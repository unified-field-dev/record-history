//! Named happy/sad contracts for product-local history APIs.
//!
//! Covers `history_row_identity`, `format_*`, `history_for_source`,
//! `resolve_history_source`, and the write→query→identity→format workflow that
//! backs the Layer 2 e2e waiver in `docs/VERIFICATION.md`.
#![cfg(feature = "ssr")]
#![allow(missing_docs)]

mod helpers;

use chrono::Utc;
use helpers::{
    as_user, create_fixture_alt_row, create_fixture_row, seed_sources, seed_user, setup_valence,
    source_a_record_id, source_b_record_id,
};
use record_history::{
    format_change_line, format_line, history_for_source, history_row_identity,
    resolve_history_source, E2eRecordHistoryFixture, RecordHistoryFields, ResolvedHistorySource,
};
use valence::{Model, RecordId};

#[test]
fn history_row_identity_splits_kind_and_row_id_happy_path() {
    let id = RecordId::new("e2e_record_history_fixture", "row-42");
    assert_eq!(
        history_row_identity(&id),
        ("e2e_record_history_fixture", "row-42")
    );

    let alt = RecordId::new("e2e_record_history_fixture_alt", "alt-7");
    assert_eq!(
        history_row_identity(&alt),
        ("e2e_record_history_fixture_alt", "alt-7")
    );
}

#[test]
fn history_row_identity_preserves_empty_row_id_sad() {
    // No validation — empty bare PK still splits so callers can reject upstream.
    let id = RecordId::new("e2e_record_history_fixture", "");
    assert_eq!(
        history_row_identity(&id),
        ("e2e_record_history_fixture", "")
    );
}

#[test]
fn format_change_line_field_diff_happy_path() {
    let row = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "name".to_string(),
        "Office".to_string(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");

    assert_eq!(
        format_change_line(&row),
        "changed name from \"Office\" to \"Office Supplies\""
    );
}

#[test]
fn format_line_includes_actor_prefix_happy_path() {
    let row = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "status".to_string(),
        "draft".to_string(),
        "active".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");

    let line = format_line(&row, "Alice Chen", Some("/user/alice"));
    assert_eq!(
        line,
        "[Alice Chen] changed status from \"draft\" to \"active\""
    );
}

#[test]
fn format_change_line_created_and_deleted_happy_path() {
    let created = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "created".to_string(),
        String::new(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");
    assert_eq!(format_change_line(&created), "created");

    let deleted = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "deleted".to_string(),
        String::new(),
        "Office Supplies".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");
    assert_eq!(format_change_line(&deleted), "deleted \"Office Supplies\"");
}

#[test]
fn format_change_line_edge_add_and_remove_happy_path() {
    let added = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "member_users".to_string(),
        String::new(),
        "user:9f3a".to_string(),
        Utc::now(),
        None,
    )
    .expect("new");
    assert_eq!(
        format_change_line(&added),
        "added member_users: \"user:9f3a\""
    );

    let removed = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "granted_users".to_string(),
        "user:1a2b".to_string(),
        String::new(),
        Utc::now(),
        None,
    )
    .expect("new");
    assert_eq!(
        format_change_line(&removed),
        "removed granted_users: \"user:1a2b\""
    );
}

#[test]
fn format_change_line_truncates_overlong_values_sad() {
    let long = "x".repeat(100);
    let row = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "description".to_string(),
        "short".to_string(),
        long.clone(),
        Utc::now(),
        None,
    )
    .expect("new");

    let line = format_change_line(&row);
    assert!(line.contains("changed description"));
    assert!(line.contains("..."));
    assert!(!line.contains(&long));
    assert_eq!(line.chars().filter(|c| *c == 'x').count(), 80);
}

#[test]
fn format_change_line_truncates_edge_add_new_value_sad() {
    let long = "y".repeat(100);
    let row = E2eRecordHistoryFixture::new(
        source_a_record_id(),
        "member_users".to_string(),
        String::new(),
        long.clone(),
        Utc::now(),
        None,
    )
    .expect("new");

    let line = format_change_line(&row);
    assert!(line.starts_with("added member_users:"));
    assert!(line.contains("..."));
    assert!(!line.contains(&long));
}

#[tokio::test]
async fn history_for_source_returns_union_rows_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    seed_user("contract-actor", "contract@example.com", &valence).await;
    let source = source_a_record_id();
    let actor = RecordId::new("user", "contract-actor");

    create_fixture_row(
        &valence,
        "contract-a",
        source.clone(),
        "name",
        "",
        "from fixture",
        1,
        Some(actor.clone()),
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "contract-alt",
        source.clone(),
        "status",
        "",
        "from alt",
        0,
        Some(actor),
        Some("note".into()),
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
}

#[tokio::test]
async fn history_for_source_unknown_source_denied_sad() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    create_fixture_row(
        &valence,
        "contract-on-a",
        source_a_record_id(),
        "name",
        "",
        "only a",
        0,
        None,
    )
    .await;

    let missing = RecordId::new("e2e_history_source_a", "does-not-exist");
    let err = history_for_source(&missing, &valence)
        .await
        .expect_err("missing parent must fail closed");
    assert!(
        err.is_access_denied(),
        "expected AccessDenied for missing source, got {err}"
    );
    assert_eq!(err.to_string(), record_history::HISTORY_ACCESS_DENIED);

    let other = history_for_source(&source_b_record_id(), &valence)
        .await
        .expect("history_for_source b");
    assert!(
        other.is_empty(),
        "source B must not see source A history, got {other:?}"
    );
}

#[tokio::test]
async fn resolve_history_source_returns_concrete_fixture_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    match resolve_history_source(&source_a_record_id(), &valence)
        .await
        .expect("resolve a")
        .expect("some a")
    {
        ResolvedHistorySource::A(row) => assert_eq!(row.label(), "Source A"),
        ResolvedHistorySource::B(_) => panic!("expected source A"),
    }

    match resolve_history_source(&source_b_record_id(), &valence)
        .await
        .expect("resolve b")
        .expect("some b")
    {
        ResolvedHistorySource::B(row) => assert_eq!(row.label(), "Source B"),
        ResolvedHistorySource::A(_) => panic!("expected source B"),
    }
}

#[tokio::test]
async fn resolve_history_source_unknown_table_or_missing_id_none_sad() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;

    let unknown_table = RecordId::new("not_a_history_source", "x");
    assert!(
        resolve_history_source(&unknown_table, &valence)
            .await
            .expect("resolve")
            .is_none(),
        "non-HistorySource table must yield None"
    );

    let missing = RecordId::new("e2e_history_source_a", "missing-id");
    assert!(
        resolve_history_source(&missing, &valence)
            .await
            .expect("resolve")
            .is_none(),
        "missing fixture id must yield None"
    );
}

#[tokio::test]
async fn user_delete_history_row_blocked_sad() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    seed_user("delete-actor", "delete-actor@example.com", &valence).await;

    let actor = RecordId::new("user", "delete-actor");
    create_fixture_row(
        &valence,
        "delete-blocked",
        source_a_record_id(),
        "name",
        "-",
        "keep",
        0,
        Some(actor),
    )
    .await;

    let user_v = as_user(&valence, "delete-actor");
    let err = E2eRecordHistoryFixture::delete("delete-blocked", &user_v)
        .await
        .expect_err("history rows must reject direct user delete");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("delete")
            || msg.contains("denied")
            || msg.contains("policy")
            || msg.contains("forbidden")
            || msg.contains("not allowed")
            || msg.contains("block"),
        "expected delete-policy error, got: {err}"
    );

    let still = E2eRecordHistoryFixture::get("delete-blocked", &valence)
        .await
        .expect("get")
        .expect("row still present");
    assert_eq!(still.new_value(), "keep");
}

#[tokio::test]
async fn history_backend_workflow_write_query_identity_format_happy_path() {
    let valence = setup_valence().await;
    seed_sources(&valence).await;
    seed_user("workflow-actor", "workflow@example.com", &valence).await;

    let source = source_a_record_id();
    let actor = RecordId::new("user", "workflow-actor");

    create_fixture_row(
        &valence,
        "workflow-created",
        source.clone(),
        "created",
        "",
        "Office Supplies",
        2,
        Some(actor.clone()),
    )
    .await;
    create_fixture_row(
        &valence,
        "workflow-rename",
        source.clone(),
        "name",
        "Office",
        "Office Supplies",
        1,
        Some(actor.clone()),
    )
    .await;
    create_fixture_alt_row(
        &valence,
        "workflow-alt",
        source.clone(),
        "status",
        "draft",
        "active",
        0,
        Some(actor),
        None,
    )
    .await;

    let rows = history_for_source(&source, &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 3);

    let mut seen_fixture = false;
    let mut seen_alt = false;
    for row in &rows {
        let id = row.id.as_ref().expect("id");
        let (kind, row_id) = history_row_identity(id);
        assert_ne!(row_id, "");
        match kind {
            "e2e_record_history_fixture" => {
                seen_fixture = true;
                assert!(row_id == "workflow-created" || row_id == "workflow-rename");
            }
            "e2e_record_history_fixture_alt" => {
                seen_alt = true;
                assert_eq!(row_id, "workflow-alt");
            }
            other => panic!("unexpected history kind {other}"),
        }

        let change = format_change_line(row);
        assert_ne!(change, "");
        if row.field_name() == "created" {
            assert_eq!(change, "created");
        } else if row.field_name() == "name" {
            assert_eq!(
                change,
                "changed name from \"Office\" to \"Office Supplies\""
            );
        }
    }
    assert!(seen_fixture && seen_alt);

    let resolved = resolve_history_source(&source, &valence)
        .await
        .expect("resolve")
        .expect("source exists");
    match resolved {
        ResolvedHistorySource::A(row) => assert_eq!(row.label(), "Source A"),
        ResolvedHistorySource::B(_) => panic!("expected source A"),
    }
}
