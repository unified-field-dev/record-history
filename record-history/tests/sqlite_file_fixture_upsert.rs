//! On-disk SQLite fixture upsert (schemas use `SQLITE_ENGINE_ID`).
#![cfg(feature = "ssr")]
#![allow(missing_docs)]

use chrono::{Duration, Utc};
use record_history::{E2eHistorySourceA, E2eRecordHistoryFixture};
use std::sync::Arc;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model, RecordId,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

#[tokio::test]
async fn e2e_fixture_row_upsert_round_trips_on_sqlite_file_happy_path() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }

    let dir = std::env::temp_dir().join(format!("record-history-sqlite-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("tmpdir");
    let db_path = dir.join("fixture.db");

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect(db_path.to_string_lossy().as_ref())
            .await
            .expect("SqliteBackend::connect"),
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
            operation: "sqlite_file_fixture_upsert_test".into(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");

    // Non-null actor avoids SQLite null-RecordId readback skew on this pin.
    {
        use chrono::Utc;
        use lepton::generated::{User, UserStatus, UserUserType};
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
        .expect("user");
        User::upsert("sqlite-fixture-actor", user, &valence)
            .await
            .expect("seed actor");
    }

    let source_rid = RecordId::new("e2e_history_source_a", "e2e-history-source-001");
    let source = E2eHistorySourceA::new("E2E Timeline Source".to_string()).expect("source new");
    E2eHistorySourceA::upsert(source_rid.id(), source, &valence)
        .await
        .expect("upsert source");

    let row = E2eRecordHistoryFixture::new(
        source_rid.clone(),
        "name".to_string(),
        "Office".to_string(),
        "Office Supplies".to_string(),
        Utc::now() - Duration::hours(0),
        Some(RecordId::new("user", "sqlite-fixture-actor")),
    )
    .expect("row new");

    let upserted = E2eRecordHistoryFixture::upsert("e2e-fix-000", row, &valence)
        .await
        .expect("fixture upsert e2e-fix-000");

    assert_eq!(upserted.source(), &source_rid);
    assert_eq!(upserted.field_name(), "name");
    assert_eq!(upserted.new_value(), "Office Supplies");

    let loaded = E2eRecordHistoryFixture::get("e2e-fix-000", &valence)
        .await
        .expect("get")
        .expect("row exists on disk-backed sqlite");
    assert_eq!(loaded.new_value(), "Office Supplies");

    let _ = std::fs::remove_dir_all(&dir);
}
