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
