#![allow(
    dead_code,
    unused_imports,
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
//! Valence-codegen output for the record-history schemas (`build.rs` + `schemas/`).
//! Generated model types are not hand-documented; see `../schemas/*.rs` for the
//! source-of-truth field definitions.

use valence::privacy_policies::common::{AUTHENTICATED, BLOCK_ALL, SYSTEM_ONLY};
use valence::privacy_policies::owner::OWNER_BY_USER_FIELD;

include!(concat!(env!("OUT_DIR"), "/generated_models.rs"));
