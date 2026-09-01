//! Record-history timeline host: seed audited mutations → query → format lines
//! under protected `/history`.
//!
//! Copy surfaces for product hosts: this package's `Cargo.toml` + `main.rs`,
//! plus the product-mount dependency / Leptos sketches in the host README.
//! Oneshot path `/history` is a protected API stand-in; the embed surface stays
//! `HistoryTimeline` + `record-history-timeline` (see JSON `inventory`).
//!
//! ## When to use
//! Smoke `history_for_source` + `format_line` without mounting `HistoryTimeline`.
//!
//! ## Command
//! ```bash
//! export CARGO_BUILD_JOBS=1
//! export CARGO_TARGET_DIR=target-record-history
//! cargo run -p timeline-host
//! ```
//!
//! ## Success
//! Stdout prints `timeline_host: OK — /history deny/allow + seeded timeline`.
//!
//! ## Look next
//! Embed `<HistoryTimeline />` from `record-history-leptos` on product detail pages.

#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]
#![allow(missing_docs)]

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use lepton::generated::{User, UserStatus, UserUserType};
use record_history::{
    format_line, history_for_source, history_row_identity, E2eHistorySourceA,
    E2eRecordHistoryFixture,
};
use tower::ServiceExt;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model, RecordId,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

const SOURCE_ID: &str = "timeline-host-source-001";
/// Fixture actor — SQLite readback rejects null actor / empty old_value.
const FIXTURE_ACTOR_ID: &str = "timeline-host-actor";

#[derive(Clone)]
struct DemoSession {
    user_id: String,
}

#[derive(Clone)]
struct HostState {
    lines: Vec<String>,
    source: String,
}

async fn require_session(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if req.extensions().get::<DemoSession>().is_some() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn inject_demo_session(mut req: Request<Body>, next: Next) -> Response {
    if let Some(user) = req
        .headers()
        .get("x-demo-user")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
    {
        req.extensions_mut().insert(DemoSession { user_id: user });
    }
    next.run(req).await
}

fn source_record_id() -> RecordId {
    RecordId::new("e2e_history_source_a", SOURCE_ID)
}

async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        backend,
        &["default"],
        RegisterBackendLogicalNamesOptions::default(),
    );

    let valence = Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(valence::router_key("default", SQLITE_ENGINE_ID))
        .with_actor(Actor::System {
            operation: "timeline-host".into(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}

fn fixture_actor() -> RecordId {
    RecordId::new("user", FIXTURE_ACTOR_ID)
}

fn fixture_old_value(old_value: &str) -> String {
    if old_value.is_empty() {
        "-".to_string()
    } else {
        old_value.to_string()
    }
}

async fn seed_fixture_actor(valence: &Valence) {
    let now = Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        Some("test-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        Some(now),
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    User::upsert(FIXTURE_ACTOR_ID, user, valence)
        .await
        .expect("upsert fixture actor");
}

async fn upsert_fixture(
    valence: &Valence,
    row_id: &str,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    hours_ago: i64,
) {
    let changed_at = Utc::now() - Duration::hours(hours_ago);
    let row = E2eRecordHistoryFixture::new(
        source_record_id(),
        field_name.to_string(),
        fixture_old_value(old_value),
        new_value.to_string(),
        changed_at,
        Some(fixture_actor()),
    )
    .expect("new fixture");
    E2eRecordHistoryFixture::upsert(row_id, row, valence)
        .await
        .expect("upsert fixture");
}

async fn bootstrap_timeline() -> HostState {
    let valence = setup_valence().await;
    seed_fixture_actor(&valence).await;
    let source = E2eHistorySourceA::new("Timeline Source".into()).expect("source");
    E2eHistorySourceA::upsert(SOURCE_ID, source, &valence)
        .await
        .expect("upsert source");

    upsert_fixture(&valence, "row-created", "created", "", "Office", 3).await;
    upsert_fixture(&valence, "row-name", "name", "Office", "Office Supplies", 2).await;
    upsert_fixture(&valence, "row-status", "status", "draft", "active", 1).await;

    let rows = history_for_source(&source_record_id(), &valence)
        .await
        .expect("history_for_source");
    assert_eq!(rows.len(), 3);

    let lines: Vec<String> = rows
        .iter()
        .map(|r| {
            let (kind, _) = history_row_identity(r.id.as_ref().expect("id"));
            let _ = kind;
            format_line(r, "Alice Chen", Some("/user/alice"))
        })
        .collect();
    assert!(lines.iter().any(|l| l.contains("Office Supplies")));

    HostState {
        lines,
        source: SOURCE_ID.to_string(),
    }
}

async fn history_api(
    Extension(session): Extension<DemoSession>,
    Extension(state): Extension<HostState>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "path": "/history",
        "user": session.user_id,
        "source_id": state.source,
        "timeline": state.lines,
        // Matches record-history-leptos embed / SSR gate (not a uf_app! route).
        "inventory": {
            "component": "HistoryTimeline",
            "testid": "record-history-timeline",
            "auth_gate": "require_session",
            "server_fn": "GetRecordHistoryPage",
        },
    }))
}

fn app(state: HostState) -> Router {
    Router::new()
        .route("/history", get(history_api))
        .route_layer(from_fn(require_session))
        .layer(Extension(state))
        .layer(from_fn(inject_demo_session))
}

#[tokio::main]
async fn main() {
    let state = bootstrap_timeline().await;

    let denied = app(state.clone())
        .oneshot(
            Request::builder()
                .uri("/history")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot")
        .status();
    assert_eq!(denied, StatusCode::UNAUTHORIZED);

    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/history")
                .header("x-demo-user", "demo-ops")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(body["path"], "/history");
    assert_eq!(body["timeline"].as_array().expect("arr").len(), 3);
    assert_eq!(body["inventory"]["component"], "HistoryTimeline");
    assert_eq!(body["inventory"]["testid"], "record-history-timeline");
    assert_eq!(body["inventory"]["auth_gate"], "require_session");
    assert_eq!(body["inventory"]["server_fn"], "GetRecordHistoryPage");

    println!("timeline_host: OK — /history deny/allow + seeded timeline");
}
