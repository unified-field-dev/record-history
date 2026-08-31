//! History UI e2e host library.
#![allow(missing_docs)]

mod app;
#[cfg(feature = "ssr")]
mod e2e_valence;
mod gate_demos;
mod harness_auth_menu;
mod pages;
#[cfg(feature = "ssr")]
pub mod seed;

pub use app::{shell, App};
#[cfg(feature = "ssr")]
pub use e2e_valence::{e2e_higgs_config, e2e_router, e2e_system_valence, init_e2e_valence};
#[cfg(feature = "ssr")]
pub use gate_demos::inject_e2e_session_snapshot;
