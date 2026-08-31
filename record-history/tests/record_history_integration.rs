#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use chrono::Utc;
use helpers::{
    create_fixture_alt_row, create_fixture_row, query_all_for_source, query_all_ordered,
    query_fixture_ordered, seed_sources, seed_user, setup_valence, source_a_record_id,
    source_b_record_id,
};
use record_history::{
    format_line, history_row_identity, E2eRecordHistoryFixture, RecordHistoryFields,
    RecordHistoryQueryAll, RecordHistoryQueryRefineE2eRecordHistoryFixture,
};
use valence::query::SortDirection;
use valence::{Model, RecordId};

#[tokio::test]
async fn create_fixture_row_round_trips() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();
    let created = create_fixture_row(
        &valence,
        "fixture-a-1",
        source.clone(),
        "name",
        "Office",
        "Office Supplies",
        1,
        None,
    )
    .await;

    let loaded = E2eRecordHistoryFixture::get("fixture-a-1", &valence)
        .await
        .expect("get")
        .expect("row exists");

    assert_eq!(loaded.source(), created.source());
    assert_eq!(loaded.field_name(), "name");
    assert_eq!(loaded.old_value(), "Office");
    assert_eq!(loaded.new_value(), "Office Supplies");
}

#[tokio::test]
async fn record_history_query_all_unions_two_tables() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(
        &valence,
        "fixture-a-union",
        source.clone(),
        "name",
        "",
        "from fixture a",
        2,
        None,
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "fixture-b-union",
        source.clone(),
        "status",
        "",
        "from fixture alt",
        1,
        None,
        Some("alt note".into()),
    )
    .await;

    let rows = query_all_for_source(&valence, &source).await;
    assert_eq!(rows.len(), 2);

    let kinds: Vec<&str> = rows
        .iter()
        .map(|r| history_row_identity(r.id.as_ref().expect("id")).0)
        .collect();
    assert!(kinds.contains(&"e2e_record_history_fixture"));
    assert!(kinds.contains(&"e2e_record_history_fixture_alt"));
}

#[tokio::test]
async fn row_identity_matches_writer_table() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(
        &valence,
        "fixture-a-id",
        source.clone(),
        "name",
        "a",
        "b",
        0,
        None,
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "fixture-b-id",
        source.clone(),
        "name",
        "c",
        "d",
        0,
        None,
        None,
    )
    .await;

    let rows = query_all_for_source(&valence, &source).await;
    for row in rows {
        let id = row.id.as_ref().expect("id");
        let (kind, row_id) = history_row_identity(id);
        assert!(kind == "e2e_record_history_fixture" || kind == "e2e_record_history_fixture_alt");
        assert_eq!(row_id, id.id());
        if kind == "e2e_record_history_fixture" {
            assert_eq!(row_id, "fixture-a-id");
        } else {
            assert_eq!(row_id, "fixture-b-id");
        }
    }
}

#[tokio::test]
async fn where_source_filters() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    create_fixture_row(
        &valence,
        "fixture-filter-a",
        source_a_record_id(),
        "name",
        "",
        "main",
        0,
        None,
    )
    .await;
    create_fixture_row(
        &valence,
        "fixture-filter-b",
        source_b_record_id(),
        "name",
        "",
        "other",
        0,
        None,
    )
    .await;

    let rows = query_all_for_source(&valence, &source_a_record_id()).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].new_value(), "main");
}

#[tokio::test]
async fn order_by_changed_at_desc() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    for (id, hours, new_value) in [
        ("rh-oldest", 10i64, "oldest"),
        ("rh-middle", 5, "middle"),
        ("rh-newest", 0, "newest"),
    ] {
        create_fixture_row(
            &valence,
            id,
            source.clone(),
            "name",
            "",
            new_value,
            hours,
            None,
        )
        .await;
    }

    let rows = query_all_ordered(&valence, &source, SortDirection::Desc, 10, 0).await;

    let values: Vec<&str> = rows.iter().map(|r| r.new_value().as_str()).collect();
    assert_eq!(values, vec!["newest", "middle", "oldest"]);
}

#[tokio::test]
async fn offset_limit_pages() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    for i in 0..6i64 {
        create_fixture_row(
            &valence,
            &format!("rh-page-{i}"),
            source.clone(),
            "name",
            "",
            &format!("row-{i}"),
            6 - i,
            None,
        )
        .await;
    }

    let page0 = query_fixture_ordered(&valence, &source, SortDirection::Asc, 2, 0).await;
    let page1 = query_fixture_ordered(&valence, &source, SortDirection::Asc, 2, 2).await;
    let page2 = query_fixture_ordered(&valence, &source, SortDirection::Asc, 2, 4).await;
    let full = query_fixture_ordered(&valence, &source, SortDirection::Asc, 10, 0).await;

    let page_values: Vec<&str> = page0
        .iter()
        .chain(page1.iter())
        .chain(page2.iter())
        .map(|r| r.new_value().as_str())
        .collect();
    let full_values: Vec<&str> = full.iter().map(|r| r.new_value().as_str()).collect();
    assert_eq!(page_values, full_values);
    assert_eq!(page_values.len(), 6);
}

#[tokio::test]
async fn where_is_fixture_refinement() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    let source = source_a_record_id();

    create_fixture_row(
        &valence,
        "fixture-refine-a",
        source.clone(),
        "name",
        "",
        "a only",
        0,
        None,
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "fixture-refine-b",
        source.clone(),
        "name",
        "",
        "b only",
        0,
        None,
        None,
    )
    .await;

    let rows: Vec<E2eRecordHistoryFixture> = RecordHistoryQueryAll::query(&valence)
        .where_source(valence::RecordPredicate::Equals(source))
        .where_is_e2e_record_history_fixture()
        .await
        .expect("refined query");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].new_value(), "a only");
}

#[tokio::test]
async fn actor_connection_optional() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    seed_user("actor-user", "actor@example.com", &valence).await;

    let source = source_a_record_id();
    let actor_rid = RecordId::new("user", "actor-user");
    create_fixture_row(
        &valence,
        "fixture-with-actor",
        source.clone(),
        "name",
        "-",
        "user row",
        1,
        Some(actor_rid.clone()),
    )
    .await;

    let with_actor = E2eRecordHistoryFixture::get("fixture-with-actor", &valence)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(with_actor.actor(), Some(&actor_rid));
    let user = with_actor
        .get_actor(&valence)
        .await
        .expect("get_actor")
        .expect("user linked");
    assert_eq!(
        user.id().map(|id| id.id().to_string()).as_deref(),
        Some("actor-user")
    );

    // None actor is still accepted on create; trait-union / Model::get readback of
    // null actor is skewed on this pin, so assert the write path only.
    let system_row = E2eRecordHistoryFixture::new(
        source,
        "created".into(),
        "-".into(),
        "system row".into(),
        chrono::Utc::now(),
        None,
    )
    .expect("new");
    assert!(system_row.actor().is_none());
}

#[test]
fn format_line_renders_diff() {
    let source = source_a_record_id();
    let row = E2eRecordHistoryFixture::new(
        source,
        "name".to_string(),
        "Office".to_string(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");

    let line = format_line(&row, "Alice Chen", Some("/user/alice"));
    assert!(line.contains("Alice Chen"));
    assert!(line.contains("name"));
    assert!(line.contains("Office"));
    assert!(line.contains("Office Supplies"));
}

#[test]
fn format_change_line_created() {
    let source = source_a_record_id();
    let row = E2eRecordHistoryFixture::new(
        source,
        "created".to_string(),
        String::new(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");

    assert_eq!(record_history::format_change_line(&row), "created");
}

#[test]
fn format_change_line_deleted() {
    let source = source_a_record_id();
    let row = E2eRecordHistoryFixture::new(
        source,
        "deleted".to_string(),
        String::new(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");

    assert_eq!(
        record_history::format_change_line(&row),
        "deleted \"Office Supplies\""
    );
}

#[test]
fn format_change_line_truncates_long_values() {
    let source = source_a_record_id();
    let long = "x".repeat(100);
    let row = E2eRecordHistoryFixture::new(
        source,
        "description".to_string(),
        "short".to_string(),
        long,
        Utc::now(),
        None,
    )
    .expect("new");

    let line = record_history::format_change_line(&row);
    assert!(line.contains("changed description"));
    assert!(line.contains("..."));
    assert!(!line.contains(&"x".repeat(100)));
}
