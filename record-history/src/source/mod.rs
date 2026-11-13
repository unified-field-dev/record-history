//! `HistorySource` resolution and source-scoped history reads.
//!
//! [`history_for_source`] authorizes the request actor against the parent
//! record, then loads every registered `RecordHistory` table for that source.
//! [`history_for_source_page`] reads one page, continued by [`HistoryKey`].
//! [`resolve_history_source`] stays on `valence::Result` (load-or-none, no ACL).

mod authorize;
mod page;
mod query;
mod resolve;

pub use authorize::authorize_history_source_read;
pub use page::{history_for_source_page, HistoryKey, HistoryPage, MAX_HISTORY_SCAN};
pub use query::history_for_source;
pub use resolve::{resolve_history_source, ResolvedHistorySource};
