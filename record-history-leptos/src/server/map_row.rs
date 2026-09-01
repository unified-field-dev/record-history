#[cfg(feature = "ssr")]
use crate::render::HistoryRowView;
#[cfg(feature = "ssr")]
use crate::server::resolve_actor_presentation;
#[cfg(feature = "ssr")]
use record_history::{history_row_identity, RecordHistoryFields, RecordHistoryModel};
#[cfg(feature = "ssr")]
use valence::Valence;

/// Map a raw [`RecordHistoryModel`] row into the wire-friendly [`HistoryRowView`],
/// resolving the actor label/href and formatting the default change line.
///
/// Includes raw `old_value` / `new_value` for custom renderers and FieldDiff mapping.
#[cfg(feature = "ssr")]
pub async fn into_history_row_view(
    model: RecordHistoryModel,
    valence: &Valence,
) -> anyhow::Result<HistoryRowView> {
    let id = model.id.as_ref().context("history row missing id")?;
    let (kind, row_id) = history_row_identity(id);
    let (actor_label, actor_href) =
        resolve_actor_presentation(model.actor().cloned(), valence).await?;
    let change_line = record_history::format_change_line(&model);
    let changed_at = model.changed_at().to_rfc3339();

    Ok(HistoryRowView {
        kind: kind.to_string(),
        row_id: row_id.to_string(),
        field_name: model.field_name().to_string(),
        old_value: model.old_value().to_string(),
        new_value: model.new_value().to_string(),
        changed_at,
        actor_label,
        actor_href,
        change_line,
    })
}

#[cfg(feature = "ssr")]
use anyhow::Context;
