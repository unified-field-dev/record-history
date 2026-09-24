//! Fail-closed parent Read check before history is paged.

use valence::{
    Actor, PrivacyEvaluator, PrivacyOperation, QueryCore, RecordId, SchemaRegistry, TraitRegistry,
    Valence,
};

use crate::error::{HistoryAccessDeniedReason, HistoryError};

/// Authorize a history read against the source/parent record.
///
/// Session presence is not enough. The request actor must satisfy the parent
/// table's Read policy. Missing rows, tables that are not `HistorySource`
/// implementors, and Read denials all map to [`HistoryError::AccessDenied`].
///
/// Valence evaluates `defer_to_edge` on history Read when the satellite schema
/// opts in. This helper remains the product-layer `HistorySource` / parent-Read
/// gate used by `history_for_source` and page APIs. Direct
/// [`crate::RecordHistoryQueryAll`] may still bypass this helper — prefer `history_for_source` for
/// ACL-sensitive reads.
///
/// # Errors
///
/// Returns [`HistoryError::AccessDenied`] when the parent cannot be authorized,
/// or [`HistoryError::Query`] when the System load of the parent row fails.
pub async fn authorize_history_source_read(
    source: &RecordId,
    valence: &Valence,
) -> Result<(), HistoryError> {
    let table = source.table();
    let record_id = source.id();
    tracing::debug!(
        operation = "authorize_history_source_read",
        table,
        record_id,
        "checking parent read authorization"
    );

    let allowed = TraitRegistry::global().tables_for_trait("HistorySource");
    if !allowed.contains(&table) {
        return Err(HistoryError::access_denied(
            HistoryAccessDeniedReason::UnsupportedSource,
        ));
    }

    let Some(schema) = SchemaRegistry::global().get_schema(table) else {
        return Err(HistoryError::access_denied(
            HistoryAccessDeniedReason::UnsupportedSource,
        ));
    };

    let system = valence.with_actor(Actor::System {
        operation: "record_history.parent_acl".to_string(),
    });
    // The data-use scan only lists a purpose written inline at the read. The
    // error wraps carry a copy of the same text so they do not show up as reads.
    let purpose = valence::use_!(
        r"Before we show the **history** of a record, we check that you are allowed to **read that record**. To run that check, we **load the record once as System**, the app's own internal identity that can look up any row, and use it only to decide whether the history opens or is refused. The record is **never returned or displayed**, so the result is a yes or no."
    );
    let raw = QueryCore::get_record_json(table, record_id, &system, valence::use_!(r"Before we show the **history** of a record, we check that you are allowed to **read that record**. To run that check, we **load the record once as System**, the app's own internal identity that can look up any row, and use it only to decide whether the history opens or is refused. The record is **never returned or displayed**, so the result is a yes or no."))
        .await
        .map_err(|e| HistoryError::query(e, purpose))?;
    let Some(raw) = raw else {
        return Err(HistoryError::access_denied(
            HistoryAccessDeniedReason::MissingSource,
        ));
    };

    match PrivacyEvaluator::check_entity_access(schema, PrivacyOperation::Read, &raw, valence).await
    {
        Ok(()) => Ok(()),
        Err(valence::Error::Privacy(_)) => Err(HistoryError::access_denied(
            HistoryAccessDeniedReason::ParentReadDenied,
        )),
        Err(e) => Err(HistoryError::query(e, purpose)),
    }
}
