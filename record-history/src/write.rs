//! Helpers for constructing history field / edge write payloads.
//!
//! Products still call generated `Model::create` (usually under System). These
//! helpers standardize `field_name` / `old_value` / `new_value` for scalar and
//! relation events so formatters and Orbital FieldDiff mapping stay consistent.

/// Parts needed to construct a concrete `RecordHistory` implementor row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryWriteParts {
    /// Field or synthetic relation name (`name`, `member_users`, …).
    pub field_name: String,
    /// Previous value (empty string for creates / edge adds).
    pub old_value: String,
    /// New value (empty string for deletes / edge removes).
    pub new_value: String,
}

/// Scalar field change (both sides non-empty).
#[must_use]
pub fn history_field_changed(field: &str, old_value: &str, new_value: &str) -> HistoryWriteParts {
    HistoryWriteParts {
        field_name: field.to_string(),
        old_value: old_value.to_string(),
        new_value: new_value.to_string(),
    }
}

/// Relation / edge add — empty `old_value`, target id in `new_value`.
#[must_use]
pub fn history_edge_added(field: &str, target_id: &str) -> HistoryWriteParts {
    HistoryWriteParts {
        field_name: field.to_string(),
        old_value: String::new(),
        new_value: target_id.to_string(),
    }
}

/// Relation / edge remove — target id in `old_value`, empty `new_value`.
#[must_use]
pub fn history_edge_removed(field: &str, target_id: &str) -> HistoryWriteParts {
    HistoryWriteParts {
        field_name: field.to_string(),
        old_value: target_id.to_string(),
        new_value: String::new(),
    }
}

/// Lifecycle create row (`field_name = "created"`).
#[must_use]
pub fn history_created(label: &str) -> HistoryWriteParts {
    HistoryWriteParts {
        field_name: "created".to_string(),
        old_value: String::new(),
        new_value: label.to_string(),
    }
}

/// Lifecycle delete row (`field_name = "deleted"`).
#[must_use]
pub fn history_deleted(label: &str) -> HistoryWriteParts {
    HistoryWriteParts {
        field_name: "deleted".to_string(),
        old_value: String::new(),
        new_value: label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_helpers_set_empty_sentinels() {
        let a = history_edge_added("member_users", "user:1");
        assert_eq!(a.old_value, "");
        assert_eq!(a.new_value, "user:1");
        let r = history_edge_removed("member_users", "user:1");
        assert_eq!(r.new_value, "");
        assert_eq!(r.old_value, "user:1");
    }
}
