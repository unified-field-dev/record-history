//! Shared constants for timeline paging and E2E fixtures.

use valence::RecordId;

/// Page size for timeline infinite scroll (server + client).
pub const RECORD_HISTORY_PAGE_SIZE: u32 = 25;

/// Bare PK for the platform E2E fixture parent (`e2e_history_source_a`).
pub const E2E_RECORD_HISTORY_SOURCE_ID: &str = "e2e-history-source-001";

/// Bare PK for empty-state preview (no history rows).
pub const E2E_RECORD_HISTORY_EMPTY_SOURCE_ID: &str = "e2e-history-empty";

/// Valence table name for the E2E fixture implementor.
pub const E2E_RECORD_HISTORY_KIND: &str = "e2e_record_history_fixture";

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
