//! Thin query helpers — prefer generated refined queries / `history_for_source`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use valence::{RecordId, Valence};

use crate::error::HistoryError;
use crate::generated::{RecordHistoryFields, RecordHistoryModel};
use crate::source::page::{history_for_source_page, MAX_HISTORY_SCAN};

/// Full-row wire shape for a concrete `RecordHistory` implementor.
///
/// Uses [`valence::datetime_unix`] so mem/SQLite unix-second cells deserialize
/// the same way generated models do. Trait-union projection into
/// [`RecordHistoryModel`] skips that reshape and is not used here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct HistoryRowWire {
    #[serde(default)]
    pub(super) id: Option<RecordId>,
    source: RecordId,
    field_name: String,
    old_value: String,
    new_value: String,
    #[serde(with = "valence::datetime_unix")]
    pub(super) changed_at: DateTime<Utc>,
    #[serde(default)]
    actor: Option<RecordId>,
}

impl RecordHistoryFields for HistoryRowWire {
    fn source(&self) -> &RecordId {
        &self.source
    }
    fn field_name(&self) -> &String {
        &self.field_name
    }
    fn old_value(&self) -> &String {
        &self.old_value
    }
    fn new_value(&self) -> &String {
        &self.new_value
    }
    fn changed_at(&self) -> &DateTime<Utc> {
        &self.changed_at
    }
    fn actor(&self) -> Option<&RecordId> {
        self.actor.as_ref()
    }
}

/// Map a concrete implementor row into the shared trait model.
pub(super) fn trait_model_from(
    row: &dyn RecordHistoryFields,
    id: Option<RecordId>,
    purpose: valence::DataUsePurpose,
) -> Result<RecordHistoryModel, HistoryError> {
    let value = serde_json::json!({
        "id": id,
        "source": row.source(),
        "field_name": row.field_name(),
        "old_value": row.old_value(),
        "new_value": row.new_value(),
        "changed_at": row.changed_at(),
        "actor": row.actor(),
    });
    serde_json::from_value(value)
        .map_err(valence::Error::serialization)
        .map_err(|e| HistoryError::query(e, purpose))
}

/// History rows for a source `RecordId` across every registered
/// `RecordHistory` implementor in the process [`valence::TraitRegistry`],
/// newest first, at most [`MAX_HISTORY_SCAN`] of them.
///
/// Authorizes the request actor against the parent `HistorySource` row first.
/// Then reads each table registered for the `RecordHistory` trait in bounded,
/// ordered windows (see [`history_for_source_page`], which this calls with no
/// key) and maps rows into [`RecordHistoryModel`]. Product tables (for example
/// Gauge `permission_history`) appear here when their crates are linked into
/// the same binary; platform E2E fixtures are included the same way. Use
/// [`history_for_source_page`] to page through longer histories.
///
/// ## Errors
///
/// Returns [`HistoryError::AccessDenied`] when the parent cannot be authorized
/// (missing, unsupported table, or parent Read denied). Returns
/// [`HistoryError::Query`] when a per-table query fails or a row cannot be
/// mapped into [`RecordHistoryModel`].
pub async fn history_for_source(
    source: &RecordId,
    valence: &Valence,
) -> Result<Vec<RecordHistoryModel>, HistoryError> {
    tracing::debug!(
        operation = "history_for_source",
        table = source.table(),
        record_id = source.id(),
        "listing history for source"
    );
    history_for_source_page(source, valence, None, 0, MAX_HISTORY_SCAN, None)
        .await
        .map(|page| page.rows)
}
