//! Logical embedded database name for record-history Valence schemas.

use valence::{Database, DatabaseFromEngine, SQLITE_ENGINE_ID};

/// Logical database name record-history schemas are registered under.
///
/// Shared with identity so history and identity tables can be joined in the
/// same embedded/test database without an explicit cross-database hop.
pub const DEFAULT_LOGICAL_NAME: &str = "default";

const ENGINE_ID: &str = SQLITE_ENGINE_ID;

/// [`DatabaseFromEngine`] pointing at [`DEFAULT_LOGICAL_NAME`] on the embedded SQLite engine.
pub const DEFAULT_STORAGE: DatabaseFromEngine =
    Database::from_engine(DEFAULT_LOGICAL_NAME, ENGINE_ID);

/// For test / server routers that link record-history models (same logical name as identity).
pub const EMBEDDED_SURREAL_LOGICAL_NAMES: &[&str] = &[DEFAULT_LOGICAL_NAME];
