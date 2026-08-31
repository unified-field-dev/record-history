//! Parent ACL contracts for timeline reads (SEC-HISTORY-PARENT).
#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod helpers;

use helpers::{
    as_user, create_fixture_row, owned_source_record_id, seed_owned_source, seed_sources,
    seed_user, setup_valence, source_a_record_id, OWNER_USER_ID, PEER_USER_ID,
};
use record_history::{
    authorize_history_source_read, history_for_source, HistoryAccessDeniedReason, HistoryError,
    RecordHistoryFields, HISTORY_ACCESS_DENIED,
};
use valence::{Actor, RecordId};

#[tokio::test]
async fn owner_pages_owned_source_history_happy_path() {
    let base = setup_valence().await;
    seed_owned_source(&base, OWNER_USER_ID).await;
    let source = owned_source_record_id();
    create_fixture_row(
        &base,
        "owned-row",
        source.clone(),
        "name",
        "Office",
        "Office Supplies",
        0,
        Some(RecordId::new("user", OWNER_USER_ID)),
    )
    .await;

    let owner = as_user(&base, OWNER_USER_ID);
    authorize_history_source_read(&source, &owner)
        .await
        .expect("owner may read parent");
    let rows = history_for_source(&source, &owner)
        .await
        .expect("owner history");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].field_name(), "name");
    assert_eq!(rows[0].new_value(), "Office Supplies");
}

#[tokio::test]
async fn peer_guessed_record_id_denied_sad() {
    let base = setup_valence().await;
    seed_owned_source(&base, OWNER_USER_ID).await;
    seed_user(PEER_USER_ID, "peer@example.com", &base).await;
    let source = owned_source_record_id();
    create_fixture_row(
        &base,
        "secret-row",
        source.clone(),
        "status",
        "draft",
        "secret-value",
        0,
        Some(RecordId::new("user", OWNER_USER_ID)),
    )
    .await;

    let peer = as_user(&base, PEER_USER_ID);
    let err = history_for_source(&source, &peer)
        .await
        .expect_err("peer must not page another record's history");
    match err {
        HistoryError::AccessDenied {
            reason: HistoryAccessDeniedReason::ParentReadDenied,
        } => {}
        other => panic!("expected ParentReadDenied, got {other:?}"),
    }
    assert_eq!(err.to_string(), HISTORY_ACCESS_DENIED);
    assert!(!err.to_string().contains("secret-value"));
    assert!(!err.to_string().contains(OWNER_USER_ID));
    assert!(!err.to_string().contains('@'));
}

#[tokio::test]
async fn authenticated_peer_reads_public_parent_history_happy_path() {
    let base = setup_valence().await;
    seed_sources(&base).await;
    seed_user(PEER_USER_ID, "peer@example.com", &base).await;
    let source = source_a_record_id();
    create_fixture_row(
        &base,
        "public-row",
        source.clone(),
        "name",
        "-",
        "catalog",
        0,
        Some(RecordId::new("user", PEER_USER_ID)),
    )
    .await;

    let peer = as_user(&base, PEER_USER_ID);
    let rows = history_for_source(&source, &peer)
        .await
        .expect("AUTHENTICATED parent (Tag analog) is readable");
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].new_value(), "catalog");
}

#[tokio::test]
async fn missing_source_denied_sad() {
    let base = setup_valence().await;
    seed_sources(&base).await;
    let missing = RecordId::new("e2e_history_source_a", "no-such-parent");
    let err = authorize_history_source_read(&missing, &base)
        .await
        .expect_err("missing parent");
    match err {
        HistoryError::AccessDenied {
            reason: HistoryAccessDeniedReason::MissingSource,
        } => {}
        other => panic!("expected MissingSource, got {other:?}"),
    }
}

#[tokio::test]
async fn unsupported_source_table_denied_sad() {
    let base = setup_valence().await;
    let unknown = RecordId::new("not_a_history_source", "guessed");
    let err = authorize_history_source_read(&unknown, &base)
        .await
        .expect_err("unsupported table");
    match err {
        HistoryError::AccessDenied {
            reason: HistoryAccessDeniedReason::UnsupportedSource,
        } => {}
        other => panic!("expected UnsupportedSource, got {other:?}"),
    }
}

#[tokio::test]
async fn anonymous_parent_read_denied_sad() {
    let base = setup_valence().await;
    seed_sources(&base).await;
    let anon = base.with_actor(Actor::Anonymous);
    let err = history_for_source(&source_a_record_id(), &anon)
        .await
        .expect_err("anonymous must not read AUTHENTICATED parent history");
    assert!(err.is_access_denied(), "got {err:?}");
}
