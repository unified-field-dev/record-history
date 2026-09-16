//! Shared constants for timeline paging and E2E fixtures.

use valence::RecordId;

/// Page size for timeline infinite scroll (server + client).
pub const RECORD_HISTORY_PAGE_SIZE: u32 = 25;

/// Client-visible denial message when the request actor cannot read the
/// `HistorySource` parent record. Mirrors `record_history::HISTORY_ACCESS_DENIED`.
///
/// This crate cannot depend on `record-history` under `hydrate`-only builds
/// (it is an `ssr`-only optional dependency), so the two crates' literals are
/// kept in sync by an `ssr`-gated equality test in
/// `tests/acl_message_sync.rs` rather than a shared import.
pub const HISTORY_ACL_DENIED_MSG: &str = "Not authorized to view this history";

/// Client-visible message when `get_record_history_page` has no session.
pub const HISTORY_AUTH_REQUIRED_MSG: &str = "Authentication required";

/// Bare PK for the platform E2E fixture parent (`e2e_history_source_a`).
pub const E2E_RECORD_HISTORY_SOURCE_ID: &str = "e2e-history-source-001";

/// Bare PK for empty-state preview (no history rows).
pub const E2E_RECORD_HISTORY_EMPTY_SOURCE_ID: &str = "e2e-history-empty";

/// Valence table name for the E2E fixture implementor.
pub const E2E_RECORD_HISTORY_KIND: &str = "e2e_record_history_fixture";

/// Valence table name for the second E2E fixture implementor, used to prove
/// `HistoryRenderers` dispatches by table name across more than one
/// registered kind (see `examples/history-ui-e2e`'s renderers scenario).
pub const E2E_RECORD_HISTORY_ALT_KIND: &str = "e2e_record_history_fixture_alt";

/// Number of rows inserted by `record_history_timeline_fixture`.
pub const E2E_RECORD_HISTORY_ROW_COUNT: u32 = 35;

/// `RecordId` for the seeded timeline parent (`e2e_history_source_a`).
pub fn e2e_record_history_source() -> RecordId {
    RecordId::new("e2e_history_source_a", E2E_RECORD_HISTORY_SOURCE_ID)
}

/// `RecordId` for the empty-state preview parent (`e2e_history_source_b`).
pub fn e2e_record_history_empty_source() -> RecordId {
    RecordId::new("e2e_history_source_b", E2E_RECORD_HISTORY_EMPTY_SOURCE_ID)
}
