//! Dispatch a `HistorySource` [`valence::RecordId`] to a concrete fixture model.

use valence::{Model, RecordId, Result, Valence};

use crate::generated::{E2eHistorySourceA, E2eHistorySourceB};

/// Resolved platform fixture source (product crates extend with their own dispatch).
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
            let row = E2eHistorySourceA::get(record_id.id(), valence).await?;
            Ok(row.map(ResolvedHistorySource::A))
        }
        "e2e_history_source_b" => {
            let row = E2eHistorySourceB::get(record_id.id(), valence).await?;
            Ok(row.map(ResolvedHistorySource::B))
        }
        _ => Ok(None),
    }
}
