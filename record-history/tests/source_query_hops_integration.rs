#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use helpers::{
    create_fixture_alt_row, create_fixture_row, seed_sources, setup_valence, source_a_record_id,
    TEST_SOURCE_A_ID,
};
use record_history::{
    history_for_source, history_row_identity, resolve_history_source, E2eHistorySourceA,
    RecordHistoryFields, RecordHistoryQueryRefineE2eRecordHistoryFixture, ResolvedHistorySource,
};

/// Q1 — resolve source A from history's source RecordId (covers query_source hop intent).
#[tokio::test]
async fn resolve_source_from_history_source_id_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(&valence, "q1-row", source.clone(), "f", "", "v", 0, None).await;

    let rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].source(), &source);

    match resolve_history_source(rows[0].source(), &valence)
        .await
        .expect("resolve")
        .expect("some")
    {
        ResolvedHistorySource::A(row) => assert_eq!(row.label(), "Source A"),
        ResolvedHistorySource::B(_) => panic!("expected source A"),
    }
}

/// Q2 — multiple history rows share one resolved source.
#[tokio::test]
async fn multiple_history_rows_share_one_source_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    for id in ["q2-a", "q2-b", "q2-c"] {
        create_fixture_row(&valence, id, source.clone(), "f", "", id, 0, None).await;
    }

    let rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|r| r.source() == &source));
}

/// Q4 — reverse hop + refine to fixture table.
#[tokio::test]
async fn query_record_history_refines_to_fixture_table_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    create_fixture_row(
        &valence,
        "q4-fix",
        source_a_record_id(),
        "f",
        "",
        "fixture only",
        1,
        None,
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "q4-alt",
        source_a_record_id(),
        "f",
        "",
        "alt only",
        0,
        None,
        None,
    )
    .await;

    let rows = E2eHistorySourceA::query(&valence)
        .where_label(valence::StringPredicate::Equals("Source A".to_string()))
        .query_record_history()
        .where_is_e2e_record_history_fixture()
        .await
        .expect("reverse hop");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].new_value(), "fixture only");
}

/// Q5 — `history_for_source` unions both kinds (generated unrefined reverse hop is SQLite-broken).
#[tokio::test]
async fn history_for_source_unions_kinds_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(&valence, "q5-fix", source.clone(), "f", "", "a", 1, None).await;
    create_fixture_alt_row(
        &valence,
        "q5-alt",
        source.clone(),
        "f",
        "",
        "b",
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
        .map(|r| history_row_identity(r.id.as_ref().expect("id")).0)
        .collect();
    assert!(kinds.contains(&"e2e_record_history_fixture"));
    assert!(kinds.contains(&"e2e_record_history_fixture_alt"));

    // Refined reverse hop still works for a single kind.
    let fixture_only = E2eHistorySourceA::query(&valence)
        .where_id(valence::StringPredicate::Equals(
            TEST_SOURCE_A_ID.to_string(),
        ))
        .query_record_history()
        .where_is_e2e_record_history_fixture()
        .await
        .expect("refined reverse hop");
    assert_eq!(fixture_only.len(), 1);
    assert_eq!(fixture_only[0].new_value(), "a");
}
