//! Dispatch a `HistorySource` [`valence::RecordId`] to a concrete fixture model.

use valence::{Model, RecordId, Result, Valence};

use crate::generated::{E2eHistorySourceA, E2eHistorySourceB};

/// Resolved platform fixture source.
///
/// This helper only dispatches the platform's own E2E fixture tables
/// (`e2e_history_source_a` / `e2e_history_source_b`). It is not a generic
/// extension point — no shipped product (Tag, Polaron, Finance) calls it.
/// Products read their own parent row directly with their own generated
/// `Model::get` with `use_!`, then call [`crate::history_for_source`] for the audit
/// rows — see the crate-root [Read history for a source](../index.html#read-history-for-a-source)
/// guide.
#[derive(Debug)]
pub enum ResolvedHistorySource {
    /// Row loaded from the `e2e_history_source_a` fixture table.
    A(E2eHistorySourceA),
    /// Row loaded from the `e2e_history_source_b` fixture table.
    B(E2eHistorySourceB),
}

/// Load the concrete `HistorySource` row for `record_id` when the table is a known fixture implementor.
///
/// Returns `Ok(None)` when the table is outside the `HistorySource` trait registry
/// or is an implementor this helper does not dispatch (product tables use their
/// own `Model::get`).
///
/// ## Errors
///
/// Returns `valence::Error` when a known fixture `get` fails.
pub async fn resolve_history_source(
    record_id: &RecordId,
    valence: &Valence,
) -> Result<Option<ResolvedHistorySource>> {
    let allowed = valence::TraitRegistry::global().tables_for_trait("HistorySource");
    if !allowed.iter().any(|t| *t == record_id.table()) {
        return Ok(None);
    }
    match record_id.table() {
        "e2e_history_source_a" => {
            let row = E2eHistorySourceA::get(record_id.id(), valence, valence::use_!(r"When a caller asks for the **record that a history timeline is about** and that record is record history's built-in **demo record A**, we **load it by its id** and hand it back to the caller. This lookup exists for the demos and tests that ship with record history; nothing is shown to anyone in this step, and the caller decides what to do with the record.")).await?;
            Ok(row.map(ResolvedHistorySource::A))
        }
        "e2e_history_source_b" => {
            let row = E2eHistorySourceB::get(record_id.id(), valence, valence::use_!(r"When a caller asks for the **record that a history timeline is about** and that record is record history's built-in **demo record B**, we **load it by its id** and hand it back to the caller. This lookup exists for the demos and tests that ship with record history; nothing is shown to anyone in this step, and the caller decides what to do with the record.")).await?;
            Ok(row.map(ResolvedHistorySource::B))
        }
        _ => Ok(None),
    }
}
