//! Map wire [`HistoryRowView`] rows onto Orbital [`HistoryEntry`] values.

use chrono::{DateTime, Utc};
use orbital_history::{HistoryActor, HistoryChange, HistoryEntry};
use orbital_paging::Page;

use super::HistoryRowView;

/// Convert a Valence page DTO into an Orbital timeline entry.
///
/// Mapping:
/// - `field_name == "created"` → [`HistoryChange::Created`]
/// - `field_name == "deleted"` → [`HistoryChange::Deleted`] (label from `new_value` / `change_line`)
/// - otherwise → [`HistoryChange::FieldDiff`] with full old/new strings
///
/// Invalid `changed_at` strings fall back to the Unix epoch (never panics).
#[must_use]
pub fn history_row_view_to_entry(row: HistoryRowView) -> HistoryEntry {
    let changed_at = DateTime::parse_from_rfc3339(&row.changed_at)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::UNIX_EPOCH);

    let change = map_change(&row);
    let actor = map_actor(&row.actor_label, row.actor_href);
    HistoryEntry {
        id: row.row_id.clone(),
        kind: row.kind,
        changed_at,
        actor,
        change,
    }
}

/// Map a page of [`HistoryRowView`] onto Orbital entries, preserving paging metadata.
#[must_use]
pub fn map_history_page(page: Page<HistoryRowView>) -> Page<HistoryEntry> {
    Page {
        items: page
            .items
            .into_iter()
            .map(history_row_view_to_entry)
            .collect(),
        has_more: page.has_more,
        total_count: page.total_count,
        next_request_offset: page.next_request_offset,
    }
}

fn map_actor(actor_label: &str, actor_href: Option<String>) -> HistoryActor {
    if actor_label == "System" {
        return HistoryActor::System;
    }
    let id = actor_href
        .as_deref()
        .and_then(user_id_from_href)
        .unwrap_or_else(|| "unknown".into());
    HistoryActor::User {
        id,
        display_name: actor_label.to_string(),
        href: actor_href,
    }
}

fn user_id_from_href(href: &str) -> Option<String> {
    let rest = href.strip_prefix("/user/")?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest.to_string())
}

fn map_change(row: &HistoryRowView) -> HistoryChange {
    match row.field_name.as_str() {
        "created" => HistoryChange::Created,
        "deleted" => HistoryChange::Deleted {
            label: if row.new_value.is_empty() {
                deleted_label(&row.change_line)
            } else {
                row.new_value.clone()
            },
        },
        _ => HistoryChange::FieldDiff {
            field: row.field_name.clone(),
            old_value: row.old_value.clone(),
            new_value: row.new_value.clone(),
        },
    }
}

fn deleted_label(change_line: &str) -> String {
    let trimmed = change_line
        .strip_prefix("deleted ")
        .unwrap_or(change_line)
        .trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed);
    unquoted.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use orbital_history::{HistoryActor, HistoryChange};

    fn sample_row() -> HistoryRowView {
        HistoryRowView {
            kind: "tag_history".into(),
            row_id: "row-1".into(),
            field_name: "name".into(),
            old_value: "a".into(),
            new_value: "b".into(),
            changed_at: "2026-01-01T00:00:00Z".into(),
            actor_label: "Ada".into(),
            actor_href: Some("/user/ada".into()),
            change_line: "changed name from \"a\" to \"b\"".into(),
        }
    }

    #[test]
    fn maps_field_diff_happy_path() {
        let entry = history_row_view_to_entry(sample_row());
        assert_eq!(entry.id, "row-1");
        assert_eq!(entry.kind, "tag_history");
        assert_eq!(
            entry.change,
            HistoryChange::FieldDiff {
                field: "name".into(),
                old_value: "a".into(),
                new_value: "b".into(),
            }
        );
        assert_eq!(
            entry.actor,
            HistoryActor::User {
                id: "ada".into(),
                display_name: "Ada".into(),
                href: Some("/user/ada".into()),
            }
        );
    }

    #[test]
    fn maps_edge_add_to_field_diff() {
        let mut row = sample_row();
        row.field_name = "member_users".into();
        row.old_value = String::new();
        row.new_value = "user:9f3a".into();
        row.change_line = "added member_users: \"user:9f3a\"".into();
        let entry = history_row_view_to_entry(row);
        assert_eq!(
            entry.change,
            HistoryChange::FieldDiff {
                field: "member_users".into(),
                old_value: String::new(),
                new_value: "user:9f3a".into(),
            }
        );
    }

    #[test]
    fn maps_created_and_deleted_happy_path() {
        let mut created = sample_row();
        created.field_name = "created".into();
        created.change_line = "created".into();
        created.actor_label = "System".into();
        created.actor_href = None;
        let created_entry = history_row_view_to_entry(created);
        assert_eq!(created_entry.change, HistoryChange::Created);
        assert_eq!(created_entry.actor, HistoryActor::System);

        let mut deleted = sample_row();
        deleted.field_name = "deleted".into();
        deleted.new_value = "Office".into();
        deleted.change_line = "deleted \"Office\"".into();
        let deleted_entry = history_row_view_to_entry(deleted);
        assert_eq!(
            deleted_entry.change,
            HistoryChange::Deleted {
                label: "Office".into(),
            }
        );
    }

    #[test]
    fn bad_changed_at_falls_back_to_epoch_sad_path() {
        let mut row = sample_row();
        row.changed_at = "not-a-date".into();
        let entry = history_row_view_to_entry(row);
        assert_eq!(entry.changed_at, DateTime::<Utc>::UNIX_EPOCH);
    }

    #[test]
    fn map_history_page_preserves_paging_metadata() {
        let page = Page {
            items: vec![sample_row()],
            has_more: true,
            total_count: Some(42),
            next_request_offset: Some(25),
        };
        let mapped = map_history_page(page);
        assert_eq!(mapped.items.len(), 1);
        assert!(mapped.has_more);
        assert_eq!(mapped.total_count, Some(42));
        assert_eq!(mapped.next_request_offset, Some(25));
        assert_eq!(mapped.items[0].kind, "tag_history");
    }
}
