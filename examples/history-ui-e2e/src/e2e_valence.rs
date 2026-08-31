//! Process-wide in-memory Valence + Higgs factory for Playwright.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, OnceLock};

use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, InMemoryBackend,
    RegisterBackendLogicalNamesOptions, RouterValenceFactory, RouterValenceFactoryConfig, Valence,
    ValenceFactory, MEM_ENGINE_ID, SQLITE_ENGINE_ID,
};

struct E2eState {
    system: Valence,
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct MemHiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for MemHiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

/// Build (once) the shared System Valence, router, and Higgs config.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    record_history::touch_schema_inventory();
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: host boot only; OnceLock reads this before first ownership get.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }

    let backend: Arc<dyn DatabaseBackend> = Arc::new(InMemoryBackend::new());
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        record_history::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions {
            register_alias_engine_id: Some(SQLITE_ENGINE_ID),
        },
    );
    router.register(
        valence::router_key(
            record_history::embedded_surreal::DEFAULT_LOGICAL_NAME,
            SQLITE_ENGINE_ID,
        ),
        backend,
    );
    let router = Arc::new(router);
    let default_key = valence::router_key(
        record_history::embedded_surreal::DEFAULT_LOGICAL_NAME,
        MEM_ENGINE_ID,
    );

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_history_host".into(),
        })
        .build()
        .expect("e2e Valence");

    let factory: Arc<dyn HiggsValenceFactory> =
        Arc::new(MemHiggsFactory(RouterValenceFactory::new(
            Arc::clone(&router),
            RouterValenceFactoryConfig::new(default_key)
                .actor_json_policy(external_actor_json_policy()),
        )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        system,
        router,
        higgs,
    });
    let _ = E2E_STATE.set(state);
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run before e2e accessors")
        .clone()
}

/// Shared System Valence for seed upserts.
pub fn e2e_system_valence() -> Valence {
    state().system.clone()
}

/// Shared Valence router for Axum `Extension` (Higgs `host_ctx`).
pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

/// Process-wide [`HiggsConfig`] for Leptos `provide_context`.
pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}
