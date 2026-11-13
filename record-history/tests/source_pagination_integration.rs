#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use helpers::{
    create_fixture_row, query_all_ordered, seed_sources, setup_valence, source_a_record_id,
};
use record_history::{history_for_source, RecordHistoryFields};
use valence::query::SortDirection;

/// Q6 — `history_for_source` returns all rows; ordered helper supports limit.
#[tokio::test]
async fn history_for_source_paginates_newest_first_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    for i in 0..4i64 {
        create_fixture_row(
            &valence,
            &format!("pg-{i}"),
            source.clone(),
            "f",
            "",
            &format!("v{i}"),
            4 - i,
            None,
        )
        .await;
    }

    let direct_all = query_all_ordered(&valence, &source, SortDirection::Desc, 10, 0).await;
    assert_eq!(direct_all.len(), 4);

    let nav_rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(nav_rows.len(), 4);

    let direct_page = query_all_ordered(&valence, &source, SortDirection::Desc, 2, 0).await;
    assert_eq!(direct_page.len(), 2);
    assert_eq!(direct_page[0].new_value(), "v3");
}

/// Q7 — direct `where_source` + offset/limit matches full set slices.
#[tokio::test]
async fn where_source_offset_limit_matches_full_set_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    for i in 0..6i64 {
        create_fixture_row(
            &valence,
            &format!("q7-{i}"),
            source.clone(),
            "f",
            "",
            &format!("v{i}"),
            6 - i,
            None,
        )
        .await;
    }

    let page = query_all_ordered(&valence, &source, SortDirection::Desc, 3, 2).await;
    let full = query_all_ordered(&valence, &source, SortDirection::Desc, 10, 0).await;
    let expected: Vec<_> = full
        .iter()
        .skip(2)
        .take(3)
        .map(|r| r.new_value().as_str())
        .collect();
    let got: Vec<_> = page.iter().map(|r| r.new_value().as_str()).collect();
    assert_eq!(got, expected);
}

/// Write one row in `table` (fixture or alt) at an exact `changed_at`.
async fn seed_row_at(
    valence: &valence::Valence,
    alt: bool,
    row_id: &str,
    source: &valence::RecordId,
    changed_at: chrono::DateTime<chrono::Utc>,
) {
    use record_history::{E2eRecordHistoryFixture, E2eRecordHistoryFixtureAlt};
    use valence::Model;

    let actor = Some(helpers::default_fixture_actor());
    if alt {
        let row = E2eRecordHistoryFixtureAlt::new(
            source.clone(),
            "f".into(),
            "-".into(),
            row_id.into(),
            changed_at,
            actor,
        )
        .expect("new alt row");
        E2eRecordHistoryFixtureAlt::upsert(row_id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Record History Fixture Alt** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
            .await
            .expect("upsert alt row");
    } else {
        let row = E2eRecordHistoryFixture::new(
            source.clone(),
            "f".into(),
            "-".into(),
            row_id.into(),
            changed_at,
            actor,
        )
        .expect("new fixture row");
        E2eRecordHistoryFixture::upsert(row_id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Record History Fixture** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
            .await
            .expect("upsert fixture row");
    }
}

/// Seed `n` rows for source A over both fixture tables, four rows per
/// millisecond so ties cross tables, and return `table:id` newest first.
async fn seed_many(valence: &valence::Valence, n: usize) -> Vec<String> {
    helpers::seed_user(
        helpers::DEFAULT_FIXTURE_ACTOR_ID,
        "rh-fixture-actor@example.com",
        valence,
    )
    .await;
    let source = source_a_record_id();
    let base = chrono::Utc::now();
    let mut expected = Vec::with_capacity(n);
    for i in 0..n {
        let alt = i % 3 == 0;
        let id = format!("pg-{i:05}");
        let at = base - chrono::Duration::milliseconds(i64::try_from(i / 4).expect("i"));
        seed_row_at(valence, alt, &id, &source, at).await;
        let table = if alt {
            "e2e_record_history_fixture_alt"
        } else {
            "e2e_record_history_fixture"
        };
        expected.push((at.timestamp_millis(), table.to_string(), id));
    }
    expected.sort_by(|a, b| b.cmp(a));
    expected
        .into_iter()
        .map(|(_, table, id)| format!("{table}:{id}"))
        .collect()
}

fn row_key(row: &record_history::RecordHistoryModel) -> String {
    let id = row.id.as_ref().expect("row id");
    format!("{}:{}", id.table(), id.id())
}

/// `history_for_source` reads past one Valence window (1,000 rows per table)
/// and returns rows newest first.
#[tokio::test]
async fn history_for_source_reads_past_one_window_newest_first_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let expected = seed_many(&valence, 1_600).await;

    let rows = history_for_source(&source_a_record_id(), &valence)
        .await
        .expect("history_for_source");
    let got: Vec<String> = rows.iter().map(row_key).collect();
    assert_eq!(got, expected);
}

/// Keyset pages cover every row exactly once, in timeline order, past one
/// Valence window; the offset path agrees with the keyset path.
#[tokio::test]
async fn history_for_source_page_keyset_reads_every_row_once_happy_path() {
    use record_history::{history_for_source_page, HistoryKey};

    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let expected = seed_many(&valence, 1_300).await;
    let source = source_a_record_id();

    let mut got = Vec::new();
    let mut after: Option<HistoryKey> = None;
    let mut pages = 0;
    loop {
        let page = history_for_source_page(&source, &valence, after.as_ref(), 0, 50, None)
            .await
            .expect("page");
        pages += 1;
        assert!(page.rows.len() <= 50);
        got.extend(page.rows.iter().map(row_key));
        if !page.has_more {
            assert!(page.next_key.is_none());
            break;
        }
        let next = page.next_key.expect("next key while has_more");
        assert_eq!(
            Some(&next),
            page.rows.last().and_then(HistoryKey::from_row).as_ref()
        );
        after = Some(next);
        assert!(pages < 1_000, "paging never ended");
    }
    assert_eq!(got, expected);

    let offset_page = history_for_source_page(&source, &valence, None, 1_000, 50, None)
        .await
        .expect("offset page");
    let offset_got: Vec<String> = offset_page.rows.iter().map(row_key).collect();
    assert_eq!(offset_got, expected[1_000..1_050]);
}

/// A kind filter reads only the named tables and still pages them in order.
#[tokio::test]
async fn history_for_source_page_kind_filter_happy_path() {
    use record_history::history_for_source_page;

    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let expected: Vec<String> = seed_many(&valence, 90)
        .await
        .into_iter()
        .filter(|k| k.starts_with("e2e_record_history_fixture_alt:"))
        .collect();
    let kinds = vec!["e2e_record_history_fixture_alt".to_string()];

    let mut got = Vec::new();
    let mut after = None;
    loop {
        let page = history_for_source_page(
            &source_a_record_id(),
            &valence,
            after.as_ref(),
            0,
            7,
            Some(&kinds),
        )
        .await
        .expect("page");
        got.extend(page.rows.iter().map(row_key));
        if !page.has_more {
            break;
        }
        after = page.next_key;
    }
    assert_eq!(got, expected);
}
