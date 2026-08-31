#![cfg(feature = "ssr")]
#![allow(missing_docs)]
// These are integration-test helpers, not `#[test]`-attributed functions themselves,
// so clippy's `allow-*-in-tests` config (in clippy.toml) does not apply to them.
#![allow(clippy::expect_used)]
// Shared helpers module included via `mod helpers` by multiple integration test
// binaries; not every consumer calls every fn, so per-binary dead-code warnings
// are expected and not meaningful here.
#![allow(dead_code)]

use chrono::{Duration, Utc};
use lepton::generated::{User, UserStatus, UserUserType};
use record_history::{
    E2eHistorySourceA, E2eHistorySourceB, E2eHistorySourceOwned, E2eRecordHistoryFixture,
    E2eRecordHistoryFixtureAlt, RecordHistoryFields, RecordHistoryQueryAll,
};
use std::sync::Arc;
use valence::query::SortDirection;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model, RecordId,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

pub const TEST_SOURCE_A_ID: &str = "test-history-source-001";
pub const TEST_SOURCE_B_ID: &str = "test-history-source-other";
pub const TEST_OWNED_SOURCE_ID: &str = "test-history-source-owned";
pub const OWNER_USER_ID: &str = "rh-owner-user";
pub const PEER_USER_ID: &str = "rh-peer-user";
/// Default actor for fixture writes — SQLite readback rejects null actor / empty old_value.
pub const DEFAULT_FIXTURE_ACTOR_ID: &str = "rh-fixture-actor";

pub fn source_a_record_id() -> RecordId {
    RecordId::new("e2e_history_source_a", TEST_SOURCE_A_ID)
}

pub fn source_b_record_id() -> RecordId {
    RecordId::new("e2e_history_source_b", TEST_SOURCE_B_ID)
}

pub fn owned_source_record_id() -> RecordId {
    RecordId::new("e2e_history_source_owned", TEST_OWNED_SOURCE_ID)
}

pub fn default_fixture_actor() -> RecordId {
    RecordId::new("user", DEFAULT_FIXTURE_ACTOR_ID)
}

async fn ensure_default_fixture_actor(valence: &Valence) {
    seed_user(
        DEFAULT_FIXTURE_ACTOR_ID,
        "rh-fixture-actor@example.com",
        valence,
    )
    .await;
}

/// Actor for DB fixture writes. `None` becomes the default fixture actor so trait-union
/// / `Model::get` readback does not hit null-RecordId skew on this pin.
async fn fixture_actor_for_write(valence: &Valence, actor: Option<RecordId>) -> RecordId {
    match actor {
        Some(a) => a,
        None => {
            ensure_default_fixture_actor(valence).await;
            default_fixture_actor()
        }
    }
}

fn fixture_old_value(old_value: &str) -> String {
    if old_value.is_empty() {
        "-".to_string()
    } else {
        old_value.to_string()
    }
}

pub async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    // Drop process-wide point-get cache so prior tests cannot satisfy `get` on a fresh DB.
    valence::clear_for_test();

    // Unified ownership fetch emits Surreal-shaped RETURN SQL that SQLite rejects.
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
            operation: "record_history_test".to_string(),
        })
        .build()
        .expect("build valence");
    // Boot-gated layout sync: registry tables refuse ad-hoc ADD COLUMN.
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}

/// Same DB as `base`, switched to the given user actor.
pub fn as_user(base: &Valence, user_id: &str) -> Valence {
    base.with_actor(Actor::User {
        user_id: user_id.to_string(),
    })
}

pub async fn seed_sources(valence: &Valence) {
    let a = E2eHistorySourceA::new("Source A".to_string()).expect("new source a");
    E2eHistorySourceA::upsert(TEST_SOURCE_A_ID, a, valence)
        .await
        .expect("upsert source a");
    let b = E2eHistorySourceB::new("Source B".to_string()).expect("new source b");
    E2eHistorySourceB::upsert(TEST_SOURCE_B_ID, b, valence)
        .await
        .expect("upsert source b");
}

pub async fn seed_owned_source(valence: &Valence, owner_user_id: &str) {
    seed_user(owner_user_id, "owner@example.com", valence).await;
    let owned = E2eHistorySourceOwned::new("Owned Source".to_string(), owner_user_id.to_string())
        .expect("new owned source");
    E2eHistorySourceOwned::upsert(TEST_OWNED_SOURCE_ID, owned, valence)
        .await
        .expect("upsert owned source");
}

pub async fn seed_user(id: &str, email: &str, valence: &Valence) {
    let _ = email; // email lives on AccountEmail upstream; label kept for call-site readability
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
    User::upsert(id, user, valence).await.expect("upsert user");
}

#[allow(clippy::too_many_arguments)]
pub async fn create_fixture_row(
    valence: &Valence,
    row_id: &str,
    source: RecordId,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    hours_ago: i64,
    actor: Option<RecordId>,
) -> E2eRecordHistoryFixture {
    let changed_at = Utc::now() - Duration::hours(hours_ago);
    let actor = fixture_actor_for_write(valence, actor).await;
    let row = E2eRecordHistoryFixture::new(
        source,
        field_name.to_string(),
        fixture_old_value(old_value),
        new_value.to_string(),
        changed_at,
        Some(actor),
    )
    .expect("new fixture row");
    E2eRecordHistoryFixture::upsert(row_id, row, valence)
        .await
        .expect("upsert fixture row")
}

#[allow(clippy::too_many_arguments)]
pub async fn create_fixture_alt_row(
    valence: &Valence,
    row_id: &str,
    source: RecordId,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    hours_ago: i64,
    actor: Option<RecordId>,
    _fixture_note: Option<String>,
) -> E2eRecordHistoryFixtureAlt {
    let changed_at = Utc::now() - Duration::hours(hours_ago);
    let actor = fixture_actor_for_write(valence, actor).await;
    let row = E2eRecordHistoryFixtureAlt::new(
        source,
        field_name.to_string(),
        fixture_old_value(old_value),
        new_value.to_string(),
        changed_at,
        Some(actor),
    )
    .expect("new fixture alt row");
    E2eRecordHistoryFixtureAlt::upsert(row_id, row, valence)
        .await
        .expect("upsert fixture alt row")
}

/// Execute a deletion DAG synchronously (tests only; bypasses async Chronon dispatcher).
pub async fn execute_deletion_dag(valence: &Valence, table: &str, bare_id: &str) {
    use valence::deletion::dag::DeletionDag;

    let dag = DeletionDag::compute(table, bare_id, valence)
        .await
        .expect("compute dag");
    assert!(
        dag.restrict_violations.is_empty(),
        "restrict violations: {:?}",
        dag.restrict_violations
    );
    for node in &dag.nodes {
        let backend = valence
            .backend_for_table(&node.table)
            .expect("backend for deletion step");
        backend
            .delete_record(&node.table, &node.record_id)
            .await
            .expect("delete dag node");
    }
}

pub async fn query_all_for_source(
    valence: &Valence,
    source: &RecordId,
) -> Vec<record_history::RecordHistoryModel> {
    // Prefer `history_for_source`: trait-union projected queries miss SQLite HasOne reshape.
    record_history::history_for_source(source, valence)
        .await
        .expect("history_for_source")
}

pub async fn query_all_ordered(
    valence: &Valence,
    source: &RecordId,
    direction: SortDirection,
    limit: u32,
    offset: u32,
) -> Vec<record_history::RecordHistoryModel> {
    let mut rows = query_all_for_source(valence, source).await;
    rows.sort_by(|a, b| {
        let cmp = a.changed_at().cmp(b.changed_at());
        match direction {
            SortDirection::Asc => cmp,
            SortDirection::Desc => cmp.reverse(),
        }
    });
    let start = offset as usize;
    rows.into_iter().skip(start).take(limit as usize).collect()
}

pub async fn query_fixture_ordered(
    valence: &Valence,
    source: &RecordId,
    direction: SortDirection,
    limit: u32,
    offset: u32,
) -> Vec<E2eRecordHistoryFixture> {
    use record_history::RecordHistoryQueryRefineE2eRecordHistoryFixture;

    RecordHistoryQueryAll::query(valence)
        .where_source(valence::RecordPredicate::Equals(source.clone()))
        .where_is_e2e_record_history_fixture()
        .order_by_changed_at(direction)
        .limit(limit)
        .offset(offset)
        .await
        .expect("ordered fixture query")
}
