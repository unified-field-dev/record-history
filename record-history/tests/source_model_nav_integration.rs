#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use helpers::{
    create_fixture_alt_row, create_fixture_row, seed_sources, setup_valence, source_a_record_id,
    source_b_record_id, TEST_SOURCE_A_ID,
};
use record_history::{resolve_history_source, E2eHistorySourceA, ResolvedHistorySource};
use valence::{Model, RecordId};

/// M1 — `resolve_history_source` returns concrete fixture A vs B.
#[tokio::test]
async fn resolve_source_returns_concrete_a_or_b() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    let a_rid = source_a_record_id();
    let resolved = resolve_history_source(&a_rid, &valence)
        .await
        .expect("resolve")
        .expect("some");
    match resolved {
        ResolvedHistorySource::A(row) => assert_eq!(row.label(), "Source A"),
        ResolvedHistorySource::B(_) => panic!("expected source A"),
    }

    let unknown = RecordId::new("e2e_history_source_a", "missing");
    assert!(resolve_history_source(&unknown, &valence)
        .await
        .expect("resolve")
        .is_none());
}

/// M2 — `history_for_source` unions both history tables for source A.
///
/// Generated `get_record_history` still uses projected trait-union queries that
/// miss SQLite HasOne reshape; product reads should use [`history_for_source`].
#[tokio::test]
async fn history_for_source_unions_both_tables_happy_path() {
    use record_history::history_for_source;

    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(
        &valence,
        "m2-fixture",
        source.clone(),
        "f",
        "",
        "from fixture",
        1,
        None,
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "m2-alt",
        source.clone(),
        "f",
        "",
        "from alt",
        0,
        None,
        None,
    )
    .await;

    let rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 2);
    let kinds: Vec<&str> = rows
        .iter()
        .map(|r| r.id.as_ref().expect("id").table())
        .collect();
    assert!(kinds.contains(&"e2e_record_history_fixture"));
    assert!(kinds.contains(&"e2e_record_history_fixture_alt"));
}

/// M3 — source B isolation.
#[tokio::test]
async fn get_record_history_isolated_per_source() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    create_fixture_row(
        &valence,
        "m3-on-b",
        source_b_record_id(),
        "f",
        "",
        "only b",
        0,
        None,
    )
    .await;

    let source_a = E2eHistorySourceA::get(TEST_SOURCE_A_ID, &valence)
        .await
        .expect("get")
        .expect("a");
    assert!(source_a
        .get_record_history(&valence)
        .await
        .expect("q")
        .is_empty());
}
