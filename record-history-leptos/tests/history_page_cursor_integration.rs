#![cfg(feature = "ssr")]
//! Cursor and offset paging through `load_history_page`, the body of the
//! `get_record_history_page` server fn after the session check.
//!
//! Runs against in-memory SQLite with user actors, so entry rows go through the
//! same parent-Read authorization and Valence read privacy as a real request.

mod helpers;

use chrono::{DateTime, Duration, Utc};
use orbital_paging::Page;
use record_history::{
    E2eHistorySourceOwned, E2eRecordHistoryFixture, E2eRecordHistoryFixtureAlt,
    HISTORY_ACCESS_DENIED,
};
use record_history_leptos::{load_history_page, HistoryRowView, MAX_HISTORY_PAGE_OFFSET};
use valence::{Actor, Model, RecordId, Valence};

const OWNER_ID: &str = "rhl-cursor-owner";
const PEER_ID: &str = "rhl-cursor-peer";
const OWNED_SOURCE_ID: &str = "rhl-cursor-owned-source";
const FIXTURE: &str = "e2e_record_history_fixture";
const FIXTURE_ALT: &str = "e2e_record_history_fixture_alt";

fn owned_source() -> RecordId {
    RecordId::new("e2e_history_source_owned", OWNED_SOURCE_ID)
}

fn as_user(base: &Valence, user_id: &str) -> Valence {
    base.with_actor(Actor::User {
        user_id: user_id.to_string(),
    })
}

/// Write one history row for `source` at an exact instant.
async fn seed_row_at(
    valence: &Valence,
    alt: bool,
    row_id: &str,
    source: &RecordId,
    changed_at: DateTime<Utc>,
) {
    let actor = Some(RecordId::new("user", OWNER_ID));
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

/// Seed `n` rows for `source` over both fixture tables, four per millisecond
/// so ties cross tables. Returns `kind:row_id` in timeline order.
async fn seed_rows(valence: &Valence, source: &RecordId, prefix: &str, n: usize) -> Vec<String> {
    let base = Utc::now();
    let mut expected = Vec::with_capacity(n);
    for i in 0..n {
        let alt = i % 3 == 0;
        let id = format!("{prefix}-{i:05}");
        let at = base - Duration::milliseconds(i64::try_from(i / 4).expect("i"));
        seed_row_at(valence, alt, &id, source, at).await;
        let table = if alt { FIXTURE_ALT } else { FIXTURE };
        expected.push((at.timestamp_millis(), table, id));
    }
    expected.sort_by(|a, b| b.cmp(a));
    expected
        .into_iter()
        .map(|(_, table, id)| format!("{table}:{id}"))
        .collect()
}

async fn seed_owned(base: &Valence) {
    helpers::seed_user(OWNER_ID, base).await;
    helpers::seed_user(PEER_ID, base).await;
    let owned = E2eHistorySourceOwned::new("Cursor Source".into(), OWNER_ID.into())
        .expect("new owned source");
    E2eHistorySourceOwned::upsert(OWNED_SOURCE_ID, owned, base, valence::use_!(r"**Test:** Fixture **E2e History Source Owned** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert owned source");
}

fn keys(page: &Page<HistoryRowView>) -> Vec<String> {
    page.items
        .iter()
        .map(|row| format!("{}:{}", row.kind, row.row_id))
        .collect()
}

/// Follow `next_cursor` (sending `next_request_offset` too, as the timeline does)
/// until `has_more` is false. Returns every row and the page count.
async fn read_by_cursor(
    v: &Valence,
    source: &RecordId,
    limit: u32,
    kinds: Option<&[String]>,
) -> (Vec<String>, usize) {
    let mut got = Vec::new();
    let mut offset = 0;
    let mut cursor = None;
    let mut pages = 0;
    loop {
        let page = load_history_page(source, v, offset, limit, kinds, cursor)
            .await
            .expect("history page");
        pages += 1;
        assert!(page.items.len() <= limit as usize);
        got.extend(keys(&page));
        if !page.has_more {
            assert_eq!(page.next_cursor, None);
            assert_eq!(page.total_count, Some(got.len() as u64));
            return (got, pages);
        }
        assert_eq!(page.total_count, None);
        cursor = Some(
            page.next_cursor
                .clone()
                .expect("next_cursor while has_more"),
        );
        offset = page
            .next_request_offset
            .expect("next_request_offset")
            .min(MAX_HISTORY_PAGE_OFFSET);
        assert!(pages < 10_000, "paging never ended");
    }
}

/// More entries than one Valence window and than `MAX_HISTORY_PAGE_OFFSET`:
/// the owner pages every entry once, in order, by cursor. A peer who can't read
/// the record gets the access-denied result, with or without the owner's cursor.
#[tokio::test]
async fn cursor_pages_every_entry_past_offset_cap_happy_path() {
    let base = helpers::setup_valence().await;
    seed_owned(&base).await;
    let source = owned_source();
    let total = MAX_HISTORY_PAGE_OFFSET as usize + 50;
    let expected = seed_rows(&base, &source, "cur", total).await;

    let owner = as_user(&base, OWNER_ID);
    let (got, pages) = read_by_cursor(&owner, &source, 50, None).await;
    assert_eq!(got.len(), expected.len());
    assert_eq!(got, expected);
    assert_eq!(pages, total.div_ceil(50));

    let peer = as_user(&base, PEER_ID);
    let err = load_history_page(&source, &peer, 0, 50, None, None)
        .await
        .expect_err("peer must not read the owner's history");
    assert!(err.to_string().contains(HISTORY_ACCESS_DENIED), "{err}");

    let first = load_history_page(&source, &owner, 0, 50, None, None)
        .await
        .expect("owner first page");
    let err = load_history_page(&source, &peer, 50, 50, None, first.next_cursor)
        .await
        .expect_err("a cursor grants nothing");
    assert!(err.to_string().contains(HISTORY_ACCESS_DENIED), "{err}");
}

/// Offset paging advances by the rows returned: following `next_request_offset`
/// without a cursor returns every row once (it used to skip one row per page).
#[tokio::test]
async fn offset_paging_returns_every_row_once_happy_path() {
    let base = helpers::setup_valence().await;
    seed_owned(&base).await;
    let source = owned_source();
    let expected = seed_rows(&base, &source, "off", 103).await;
    let owner = as_user(&base, OWNER_ID);

    let mut got = Vec::new();
    let mut offset = 0;
    loop {
        let page = load_history_page(&source, &owner, offset, 25, None, None)
            .await
            .expect("offset page");
        let next = page.next_request_offset.expect("next_request_offset");
        assert_eq!(next, offset + u32::try_from(page.items.len()).expect("len"));
        got.extend(keys(&page));
        if !page.has_more {
            break;
        }
        offset = next;
    }
    assert_eq!(got, expected);
}

/// A cursor from another kind filter, page size or record, or a garbage token,
/// falls back to the offset path.
#[tokio::test]
async fn stale_cursor_falls_back_to_offset_sad() {
    let base = helpers::setup_valence().await;
    seed_owned(&base).await;
    let source = owned_source();
    let expected = seed_rows(&base, &source, "stale", 120).await;
    let owner = as_user(&base, OWNER_ID);

    let first = load_history_page(&source, &owner, 0, 25, None, None)
        .await
        .expect("first page");
    let cursor = first.next_cursor.clone().expect("cursor");

    // Same query: the cursor continues after the first page.
    let next = load_history_page(&source, &owner, 0, 25, None, Some(cursor.clone()))
        .await
        .expect("cursor page");
    assert_eq!(keys(&next), expected[25..50]);

    // Different kind filter: offset 25 within the filtered rows.
    let kinds = vec![FIXTURE.to_string()];
    let filtered: Vec<String> = expected
        .iter()
        .filter(|k| k.starts_with(&format!("{FIXTURE}:")))
        .cloned()
        .collect();
    let page = load_history_page(&source, &owner, 25, 25, Some(&kinds), Some(cursor.clone()))
        .await
        .expect("filtered page");
    assert_eq!(keys(&page), filtered[25..50]);

    // Different page size: offset 30, ten rows.
    let page = load_history_page(&source, &owner, 30, 10, None, Some(cursor.clone()))
        .await
        .expect("resized page");
    assert_eq!(keys(&page), expected[30..40]);

    // Garbage tokens.
    for token in ["", "not-a-cursor", "e30"] {
        let page = load_history_page(&source, &owner, 40, 25, None, Some(token.into()))
            .await
            .expect("garbage cursor page");
        assert_eq!(keys(&page), expected[40..65], "token {token:?}");
    }

    // Another record's cursor: offset 0 of that record.
    helpers::seed_source(&base).await;
    let other = helpers::source_record_id();
    let other_expected = seed_rows(&base, &other, "other", 30).await;
    let page = load_history_page(&other, &owner, 0, 25, None, Some(cursor))
        .await
        .expect("other record page");
    assert_eq!(keys(&page), other_expected[..25]);
}
