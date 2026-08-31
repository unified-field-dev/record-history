//! Harness-only seed endpoint for Playwright.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use axum::http::StatusCode;
use axum::Json;
use chrono::{Duration, Utc};
use lepton::generated::{User, UserStatus, UserUserType};
use record_history::{
    E2eHistorySourceA, E2eHistorySourceB, E2eHistorySourceOwned, E2eRecordHistoryFixture,
    E2eRecordHistoryFixtureAlt,
};
use record_history_leptos::{
    set_e2e_history_load_fault, E2E_RECORD_HISTORY_EMPTY_SOURCE_ID, E2E_RECORD_HISTORY_ROW_COUNT,
    E2E_RECORD_HISTORY_SOURCE_ID, RECORD_HISTORY_PAGE_SIZE,
};
use serde::Deserialize;
use valence::{Actor, Model, RecordId, Valence};

use crate::e2e_valence::e2e_system_valence;
use crate::gate_demos::{write_e2e_auth_kind, E2eAuthKind};

pub const E2E_OWNER_USER_ID: &str = "e2e-user";
pub const E2E_OUTSIDER_USER_ID: &str = "e2e-outsider";
pub const E2E_OWNED_SOURCE_ID: &str = "e2e-owned-source";
pub const E2E_PUBLIC_SOURCE_ID: &str = "e2e-public-source";
pub const E2E_MULTIKIND_SOURCE_ID: &str = "e2e-history-multikind";
pub const E2E_OWNED_CHANGE: &str = "Office Supplies";
pub const E2E_PUBLIC_CHANGE: &str = "catalog-visible";
pub const E2E_FIXTURE_KIND_MARKER: &str = "fixture-kind-marker";
pub const E2E_ALT_KIND_MARKER: &str = "alt-kind-marker";
pub const E2E_PAGE_LATE_MARKER: &str = "page-row-0";

#[derive(Debug, Deserialize)]
pub struct SeedRequest {
    /// E2e auth kind: `anonymous`, `owner`, `outsider`.
    #[serde(default = "default_auth")]
    pub auth: String,
    /// Seed scenario: `default`, `empty`, `multipage`, `multikind`, `load_fail`.
    #[serde(default = "default_scenario")]
    pub scenario: String,
    /// When true (or scenario `load_fail`), arm SSR load fault for history page fetch.
    #[serde(default)]
    pub fault: bool,
}

fn default_auth() -> String {
    E2eAuthKind::Anonymous.as_str().to_string()
}

fn default_scenario() -> String {
    "default".to_string()
}

async fn seed_user(id: &str, valence: &Valence) {
    let now = Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        Some(now),
        None,
        None,
        now,
        now,
    )
    .expect("build e2e user");
    User::upsert(id, user, valence)
        .await
        .expect("upsert e2e user");
}

async fn ensure_history(
    valence: &Valence,
    row_id: &str,
    source: RecordId,
    new_value: &str,
    actor_id: &str,
    hours_ago: i64,
) {
    // History rows block update (`always_block`). Re-seed must not upsert an existing row.
    if E2eRecordHistoryFixture::get(row_id, valence)
        .await
        .expect("get history")
        .is_some()
    {
        return;
    }
    let row = E2eRecordHistoryFixture::new(
        source,
        "name".to_string(),
        "Office".to_string(),
        new_value.to_string(),
        Utc::now() - Duration::hours(hours_ago),
        Some(RecordId::new("user", actor_id)),
    )
    .expect("history row");
    E2eRecordHistoryFixture::upsert(row_id, row, valence)
        .await
        .expect("create history");
}

async fn ensure_history_alt(
    valence: &Valence,
    row_id: &str,
    source: RecordId,
    new_value: &str,
    actor_id: &str,
    hours_ago: i64,
) {
    if E2eRecordHistoryFixtureAlt::get(row_id, valence)
        .await
        .expect("get alt history")
        .is_some()
    {
        return;
    }
    let row = E2eRecordHistoryFixtureAlt::new(
        source,
        "name".to_string(),
        "Office".to_string(),
        new_value.to_string(),
        Utc::now() - Duration::hours(hours_ago),
        Some(RecordId::new("user", actor_id)),
    )
    .expect("alt history row");
    E2eRecordHistoryFixtureAlt::upsert(row_id, row, valence)
        .await
        .expect("create alt history");
}

/// Seed users, an owner-scoped parent, and an AUTHENTICATED parent (Tag analog).
pub async fn seed_users_and_fixtures(system: &Valence) {
    seed_user(E2E_OWNER_USER_ID, system).await;
    seed_user(E2E_OUTSIDER_USER_ID, system).await;

    let owned =
        E2eHistorySourceOwned::new("Owned Source".to_string(), E2E_OWNER_USER_ID.to_string())
            .expect("owned source");
    E2eHistorySourceOwned::upsert(E2E_OWNED_SOURCE_ID, owned, system)
        .await
        .expect("upsert owned source");

    let public = E2eHistorySourceA::new("Public Source".to_string()).expect("public source");
    E2eHistorySourceA::upsert(E2E_PUBLIC_SOURCE_ID, public, system)
        .await
        .expect("upsert public source");

    ensure_history(
        system,
        "e2e-owned-row",
        RecordId::new("e2e_history_source_owned", E2E_OWNED_SOURCE_ID),
        E2E_OWNED_CHANGE,
        E2E_OWNER_USER_ID,
        1,
    )
    .await;
    ensure_history(
        system,
        "e2e-public-row",
        RecordId::new("e2e_history_source_a", E2E_PUBLIC_SOURCE_ID),
        E2E_PUBLIC_CHANGE,
        E2E_OWNER_USER_ID,
        1,
    )
    .await;
}

async fn seed_empty_parent(system: &Valence) {
    let empty = E2eHistorySourceB::new("Empty Source".to_string()).expect("empty source");
    E2eHistorySourceB::upsert(E2E_RECORD_HISTORY_EMPTY_SOURCE_ID, empty, system)
        .await
        .expect("upsert empty source");
}

async fn seed_multipage_parent(system: &Valence) {
    let source = E2eHistorySourceA::new("Multipage Source".to_string()).expect("multipage source");
    E2eHistorySourceA::upsert(E2E_RECORD_HISTORY_SOURCE_ID, source, system)
        .await
        .expect("upsert multipage source");
    let parent = RecordId::new("e2e_history_source_a", E2E_RECORD_HISTORY_SOURCE_ID);
    for i in 0..E2E_RECORD_HISTORY_ROW_COUNT {
        let marker = format!("page-row-{i}");
        ensure_history(
            system,
            &format!("e2e-page-row-{i}"),
            parent.clone(),
            &marker,
            E2E_OWNER_USER_ID,
            i64::from(E2E_RECORD_HISTORY_ROW_COUNT - i),
        )
        .await;
    }
}

async fn seed_multikind_parent(system: &Valence) {
    let source = E2eHistorySourceA::new("Multikind Source".to_string()).expect("multikind source");
    E2eHistorySourceA::upsert(E2E_MULTIKIND_SOURCE_ID, source, system)
        .await
        .expect("upsert multikind source");
    let parent = RecordId::new("e2e_history_source_a", E2E_MULTIKIND_SOURCE_ID);
    ensure_history(
        system,
        "e2e-multikind-fixture",
        parent.clone(),
        E2E_FIXTURE_KIND_MARKER,
        E2E_OWNER_USER_ID,
        2,
    )
    .await;
    ensure_history_alt(
        system,
        "e2e-multikind-alt",
        parent,
        E2E_ALT_KIND_MARKER,
        E2E_OWNER_USER_ID,
        1,
    )
    .await;
}

pub async fn seed_data(
    session: tower_sessions::Session,
    Json(body): Json<SeedRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let kind = E2eAuthKind::parse(&body.auth);
    write_e2e_auth_kind(&session, kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let system = e2e_system_valence().with_actor(Actor::System {
        operation: "e2e_seed_history".into(),
    });
    seed_users_and_fixtures(&system).await;

    let scenario = body.scenario.as_str();
    let arm_fault = body.fault || scenario == "load_fail";
    set_e2e_history_load_fault(arm_fault);

    match scenario {
        "empty" => seed_empty_parent(&system).await,
        "multipage" => seed_multipage_parent(&system).await,
        "multikind" => seed_multikind_parent(&system).await,
        "load_fail" | "default" => {}
        other => {
            log::warn!("unknown history-ui-e2e seed scenario `{other}`; using default");
        }
    }

    Ok(Json(serde_json::json!({
        "ok": true,
        "auth": kind.as_str(),
        "scenario": scenario,
        "fault": arm_fault,
        "owned_source_id": E2E_OWNED_SOURCE_ID,
        "public_source_id": E2E_PUBLIC_SOURCE_ID,
        "owned_change": E2E_OWNED_CHANGE,
        "public_change": E2E_PUBLIC_CHANGE,
        "empty_source_id": E2E_RECORD_HISTORY_EMPTY_SOURCE_ID,
        "multipage_source_id": E2E_RECORD_HISTORY_SOURCE_ID,
        "multikind_source_id": E2E_MULTIKIND_SOURCE_ID,
        "page_size": RECORD_HISTORY_PAGE_SIZE,
        "row_count": E2E_RECORD_HISTORY_ROW_COUNT,
        "fixture_kind_marker": E2E_FIXTURE_KIND_MARKER,
        "alt_kind_marker": E2E_ALT_KIND_MARKER,
        "page_late_marker": E2E_PAGE_LATE_MARKER,
    })))
}
