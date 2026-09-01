//! Process-local fault flag for the history-ui-e2e harness only.
//!
//! Enabled via Cargo feature `e2e-harness`. Cleared on default seed so ACL
//! Playwright scenarios never inherit a sticky fault.

use std::sync::atomic::{AtomicBool, Ordering};

static LOAD_FAULT: AtomicBool = AtomicBool::new(false);

/// Arm or clear the forced load-failure for [`crate::get_record_history_page`].
pub fn set_e2e_history_load_fault(on: bool) {
    LOAD_FAULT.store(on, Ordering::SeqCst);
}

/// True when the next history page fetch should return `Failed to load record history`.
#[must_use]
pub fn e2e_history_load_fault_active() -> bool {
    LOAD_FAULT.load(Ordering::SeqCst)
}
