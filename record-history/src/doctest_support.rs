//! Doctest-only Valence bootstrap.
//!
//! `#[doc(hidden)]`, gated behind the non-default `doctest-support` feature.
//! Never enabled by a product build — only by `cargo test --doc`. Mirrors
//! `tests/helpers.rs::setup_valence()` so rustdoc examples that need a live
//! `Valence` (rather than a pure-function example) can actually run.

use std::sync::Arc;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

/// Build an in-memory SQLite [`Valence`] for a doctest.
///
/// # Panics
///
/// Panics on fixture-setup failure (connecting the in-memory backend or
/// syncing typed tables). This is a test-only bootstrap, not a production
/// API: a panic here means the doctest harness itself is broken, which
/// should fail loudly rather than silently skip the example.
#[doc(hidden)]
pub async fn doctest_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();

    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: doctest harness only; OnceLock reads this before first ownership get.
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
            operation: "record_history_doctest".to_string(),
        })
        .build()
        .expect("build valence");
    valence
        .sync_typed_tables_from_registry()
        .await
        .expect("sync_typed_tables_from_registry");
    valence
}
