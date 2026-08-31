use serde::{Deserialize, Serialize};

/// Wire type for a single audit row in the timeline (SSR → client).
///
/// Carries a preformatted [`change_line`](Self::change_line) for stock UI plus
/// raw [`old_value`](Self::old_value) / [`new_value`](Self::new_value) so product
/// [`HistoryRenderers`](orbital_history::HistoryRenderers) can build Orbital
/// [`FieldDiff`](orbital_history::HistoryChange::FieldDiff) or Avatar rows without
/// a second Valence round-trip. [`kind`](Self::kind) and [`row_id`](Self::row_id)
/// remain for renderer routing — see crate-root Register history renderers guide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRowView {
    /// Concrete Valence table name (`tag_history`, `e2e_record_history_fixture`, …).
    pub kind: String,
    /// Bare history-row PK (for custom renderers that load a concrete model).
    pub row_id: String,
    /// Name of the field that changed (`"created"`/`"deleted"` for lifecycle rows).
    pub field_name: String,
    /// Previous value (may be empty for creates / edge adds).
    pub old_value: String,
    /// New value (may be empty for deletes / edge removes).
    pub new_value: String,
    /// ISO-8601 timestamp for display.
    pub changed_at: String,
    /// Resolved actor label (`[Display Name]`, opaque `User …`, or `System`).
    pub actor_label: String,
    /// `/user/{id}` when the actor connection is set.
    pub actor_href: Option<String>,
    /// Default line from [`record_history::format_change_line`].
    pub change_line: String,
}
