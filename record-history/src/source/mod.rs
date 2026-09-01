//! `HistorySource` resolution and source-scoped history reads.
//!
//! [`history_for_source`] authorizes the request actor against the parent
//! record, then loads every registered `RecordHistory` table for that source.
//! [`resolve_history_source`] stays on `valence::Result` (load-or-none, no ACL).

mod authorize;
mod query;
mod resolve;

pub use authorize::authorize_history_source_read;
pub use query::history_for_source;
pub use resolve::{resolve_history_source, ResolvedHistorySource};
