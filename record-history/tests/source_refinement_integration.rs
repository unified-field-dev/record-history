#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use helpers::{
    create_fixture_row, seed_sources, setup_valence, source_a_record_id, TEST_SOURCE_A_ID,
};
use record_history::{
    history_for_source, resolve_history_source, E2eHistorySourceA, E2eHistorySourceB,
    HistorySourceQueryAll, HistorySourceQueryRefineE2eHistorySourceA,
    HistorySourceQueryRefineE2eHistorySourceB, RecordHistoryFields, RecordHistoryQueryAll,
    RecordHistoryQueryRefineE2eRecordHistoryFixture, ResolvedHistorySource,
};

/// Q3 — history rows resolve back to source A (covers kind→source hop intent).
#[tokio::test]
async fn history_rows_resolve_to_source_a_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(&valence, "q3-fix", source.clone(), "f", "", "v", 0, None).await;

    let rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 1);

    match resolve_history_source(rows[0].source(), &valence)
        .await
        .expect("resolve")
        .expect("some")
    {
        ResolvedHistorySource::A(row) => assert_eq!(row.label(), "Source A"),
        ResolvedHistorySource::B(_) => panic!("expected source A"),
    }
}

/// B2 — `HistorySourceQueryAll` refinement narrows to one implementor table.
#[tokio::test]
async fn history_source_query_all_refine_a_vs_b_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    let a_rows: Vec<E2eHistorySourceA> = HistorySourceQueryAll::query(&valence)
        .where_is_e2e_history_source_a()
        .await
        .expect("refine a");
    let b_rows: Vec<E2eHistorySourceB> = HistorySourceQueryAll::query(&valence)
        .where_is_e2e_history_source_b()
        .await
        .expect("refine b");

    assert_eq!(a_rows.len(), 1);
    assert_eq!(b_rows.len(), 1);
    assert_eq!(a_rows[0].label(), "Source A");
    assert_eq!(b_rows[0].label(), "Source B");
}

/// Q7 — direct filter without hop matches refined reverse-hop path.
#[tokio::test]
async fn direct_where_source_and_where_is_kind_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(
        &valence,
        "q7-row",
        source.clone(),
        "f",
        "",
        "direct",
        0,
        None,
    )
    .await;

    let direct: Vec<_> = RecordHistoryQueryAll::query(&valence)
        .where_source(valence::RecordPredicate::Equals(source.clone()))
        .where_is_e2e_record_history_fixture()
        .order_by_changed_at(valence::query::SortDirection::Desc)
        .await
        .expect("direct");

    let hop: Vec<_> = E2eHistorySourceA::query(&valence)
        .where_id(valence::StringPredicate::Equals(
            TEST_SOURCE_A_ID.to_string(),
        ))
        .query_record_history()
        .where_is_e2e_record_history_fixture()
        .await
        .expect("hop");

    assert_eq!(direct.len(), hop.len());
    assert_eq!(direct[0].new_value(), hop[0].new_value());
}
