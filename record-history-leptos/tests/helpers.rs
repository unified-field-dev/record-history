#![cfg(feature = "ssr")]
#![allow(missing_docs)]
// Integration-test helpers, not `#[test]`-attributed functions themselves, so
// clippy's `allow-*-in-tests` config does not apply here.
#![allow(clippy::expect_used)]
// Shared helpers module included via `mod helpers` by multiple integration test
// binaries; not every consumer calls every fn.
#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use lepton::generated::{User, UserStatus, UserUserType};
use record_history::{E2eHistorySourceA, E2eRecordHistoryFixture, E2eRecordHistoryFixtureAlt};
use std::sync::Arc;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model, RecordId,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

pub const TEST_SOURCE_ID: &str = "rhl-test-source-001";
pub const NAMED_ACTOR_ID: &str = "rhl-named-actor";

pub fn source_record_id() -> RecordId {
    RecordId::new("e2e_history_source_a", TEST_SOURCE_ID)
}

/// Adapted from `record-history/tests/helpers.rs::setup_valence` — this crate's
/// integration tests need the same in-memory SQLite Valence bootstrap but live
/// in a separate crate's `tests/` dir, so the setup is duplicated rather than
/// shared (no public test-helper crate exists to import from).
pub async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();

    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only; OnceLock reads this before first ownership get.
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
            operation: "record_history_leptos_test".to_string(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}

pub async fn seed_user(id: &str, valence: &Valence) {
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
    User::upsert(id, user, valence, valence::use_!(r"**Test:** Fixture **User** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only.")).await.expect("upsert user");
}

pub async fn seed_source(valence: &Valence) {
    let source = E2eHistorySourceA::new("RHL Test Source".to_string()).expect("new source");
    E2eHistorySourceA::upsert(TEST_SOURCE_ID, source, valence, valence::use_!(r"**Test:** Fixture **E2e History Source A** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert source");
}

#[allow(clippy::too_many_arguments)]
pub async fn create_fixture_row(
    valence: &Valence,
    row_id: &str,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    changed_at: DateTime<Utc>,
    actor: Option<RecordId>,
) {
    let row = E2eRecordHistoryFixture::new(
        source_record_id(),
        field_name.to_string(),
        old_value.to_string(),
        new_value.to_string(),
        changed_at,
        actor,
    )
    .expect("new fixture row");
    E2eRecordHistoryFixture::upsert(row_id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Record History Fixture** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert fixture row");
}

#[allow(clippy::too_many_arguments)]
pub async fn create_fixture_alt_row(
    valence: &Valence,
    row_id: &str,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    changed_at: DateTime<Utc>,
    actor: Option<RecordId>,
) {
    let row = E2eRecordHistoryFixtureAlt::new(
        source_record_id(),
        field_name.to_string(),
        old_value.to_string(),
        new_value.to_string(),
        changed_at,
        actor,
    )
    .expect("new fixture alt row");
    E2eRecordHistoryFixtureAlt::upsert(row_id, row, valence, valence::use_!(r"**Test:** Fixture **E2e Record History Fixture Alt** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("upsert fixture alt row");
}

pub fn hours_ago(hours: i64) -> DateTime<Utc> {
    Utc::now() - Duration::hours(hours)
}
