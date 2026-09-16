#![cfg(feature = "ssr")]
//! Integration coverage for `record-history-leptos`'s SSR row mapper
//! (`into_history_row_view`) and the pure `HistoryRowView -> HistoryEntry`
//! mapper, against real DB-written rows.
//!
//! Before this file, `record-history-leptos` had no crate-level tests of its
//! own: `into_history_row_view` (the function that turns a raw Valence model
//! into the wire DTO every product's timeline renders) was only ever
//! exercised transitively through the full-stack Playwright suite against the
//! platform's own E2E fixtures. These tests close that gap at the lowest
//! effective layer.

mod helpers;

use helpers::{
    create_fixture_alt_row, create_fixture_row, hours_ago, seed_source, seed_user, setup_valence,
    source_record_id, NAMED_ACTOR_ID,
};
use orbital_history::HistoryChange;
use record_history::history_for_source;
use record_history_leptos::history_row_view_to_entry;
use record_history_leptos::server::into_history_row_view;
use valence::RecordId;

async fn only_row(valence: &valence::Valence) -> record_history::RecordHistoryModel {
    let mut rows = history_for_source(&source_record_id(), valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 1, "expected exactly one history row for source");
    rows.pop().expect("row present")
}

#[tokio::test]
async fn into_history_row_view_maps_field_change_happy_path() {
    let valence = setup_valence().await;
    seed_source(&valence).await;
    seed_user(NAMED_ACTOR_ID, &valence).await;

    create_fixture_row(
        &valence,
        "rhl-row-field-change",
        "name",
        "Office",
        "Office Supplies",
        hours_ago(1),
        Some(RecordId::new("user", NAMED_ACTOR_ID)),
    )
    .await;

    let model = only_row(&valence).await;
    let view = into_history_row_view(model, &valence)
        .await
        .expect("into_history_row_view");

    assert_eq!(view.kind, "e2e_record_history_fixture");
    assert_eq!(view.field_name, "name");
    assert_eq!(view.old_value, "Office");
    assert_eq!(view.new_value, "Office Supplies");
    assert_ne!(view.actor_label, "System");
    assert!(
        view.actor_href.as_deref() == Some(&format!("/user/{NAMED_ACTOR_ID}")[..]),
        "expected safe actor href, got {:?}",
        view.actor_href
    );
    assert!(
        view.change_line.contains("Office Supplies"),
        "change_line should surface the new value: {}",
        view.change_line
    );
}

#[tokio::test]
async fn into_history_row_view_maps_alt_kind_and_system_actor_happy_path() {
    let valence = setup_valence().await;
    seed_source(&valence).await;

    create_fixture_alt_row(
        &valence,
        "rhl-row-alt-system",
        "status",
        "draft",
        "published",
        hours_ago(2),
        None, // actor: None => System, proving the mapper is table-name-agnostic
    )
    .await;

    let model = only_row(&valence).await;
    let view = into_history_row_view(model, &valence)
        .await
        .expect("into_history_row_view");

    assert_eq!(
        view.kind, "e2e_record_history_fixture_alt",
        "mapper must resolve the concrete table name, not just the platform's primary fixture table"
    );
    assert_eq!(view.field_name, "status");
    assert_eq!(view.actor_label, "System");
    assert_eq!(view.actor_href, None);
}

#[tokio::test]
async fn into_history_row_view_missing_actor_user_falls_back_opaque_sad() {
    let valence = setup_valence().await;
    seed_source(&valence).await;
    // Actor id is never seeded via seed_user — simulates a deleted/unknown user.

    create_fixture_row(
        &valence,
        "rhl-row-ghost-actor",
        "name",
        "Office",
        "Office Supplies",
        hours_ago(1),
        Some(RecordId::new("user", "rhl-ghost-actor-id")),
    )
    .await;

    let model = only_row(&valence).await;
    let view = into_history_row_view(model, &valence)
        .await
        .expect("into_history_row_view must not fail on an unresolvable actor");

    assert!(
        view.actor_label.starts_with("User "),
        "unresolvable actor must fall back to an opaque label, got {:?}",
        view.actor_label
    );
    assert!(
        !view.actor_label.contains('@'),
        "actor label must never leak an email-shaped string: {:?}",
        view.actor_label
    );
}

#[tokio::test]
async fn history_row_view_to_entry_round_trip_with_live_row_happy_path() {
    let valence = setup_valence().await;
    seed_source(&valence).await;

    let created = record_history::history_created("Created Label");
    create_fixture_row(
        &valence,
        "rhl-row-created",
        &created.field_name,
        &created.old_value,
        &created.new_value,
        hours_ago(3),
        None,
    )
    .await;

    let deleted = record_history::history_deleted("Deleted Label");
    create_fixture_alt_row(
        &valence,
        "rhl-row-deleted",
        &deleted.field_name,
        &deleted.old_value,
        &deleted.new_value,
        hours_ago(1),
        None,
    )
    .await;

    let rows = history_for_source(&source_record_id(), &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 2);

    let mut saw_created = false;
    let mut saw_deleted = false;
    for model in rows {
        let view = into_history_row_view(model, &valence)
            .await
            .expect("into_history_row_view");
        let entry = history_row_view_to_entry(view);
        match entry.change {
            HistoryChange::Created => saw_created = true,
            HistoryChange::Deleted { ref label } => {
                assert_eq!(label, "Deleted Label");
                saw_deleted = true;
            }
            other => panic!("unexpected change variant from a live DB row: {other:?}"),
        }
    }
    assert!(saw_created, "expected a Created entry from the live row");
    assert!(saw_deleted, "expected a Deleted entry from the live row");
}
