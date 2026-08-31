//! Default timeline line formatting from trait fields.
//!
//! These helpers are pure and infallible — no `Result`, no I/O.
//!
//! ## Field-name conventions
//!
//! | `field_name` | `old_value` / `new_value` | `format_change_line` |
//! |---|---|---|
//! | `created` | often empty / label | `created` |
//! | `deleted` | empty / previous label | `deleted "{new}"` |
//! | any (edge add) | empty old, non-empty new | `added {field}: "{new}"` |
//! | any (edge remove) | non-empty old, empty new | `removed {field}: "{old}"` |
//! | any (scalar) | both non-empty | `changed {field} from "{old}" to "{new}"` |

use crate::generated::RecordHistoryFields;

const DISPLAY_VALUE_MAX_LEN: usize = 80;

fn truncate_display(value: &str) -> String {
    if value.chars().count() <= DISPLAY_VALUE_MAX_LEN {
        return value.to_string();
    }
    let truncated: String = value.chars().take(DISPLAY_VALUE_MAX_LEN).collect();
    format!("{truncated}...")
}

/// Change-line text only (no actor prefix) for timeline UI (`change_line` on the
/// page DTO / Orbital change region).
#[must_use]
pub fn format_change_line(row: &dyn RecordHistoryFields) -> String {
    let field = row.field_name();
    let old_raw = row.old_value();
    let new_raw = row.new_value();
    let old = truncate_display(old_raw);
    let new = truncate_display(new_raw);

    match field.as_str() {
        "created" => "created".to_string(),
        "deleted" => format!("deleted \"{new}\""),
        _ if old_raw.is_empty() && !new_raw.is_empty() => {
            format!("added {field}: \"{new}\"")
        }
        _ if !old_raw.is_empty() && new_raw.is_empty() => {
            format!("removed {field}: \"{old}\"")
        }
        _ => format!("changed {field} from \"{old}\" to \"{new}\""),
    }
}

/// Format a full audit line including actor: `[Actor] changed field from "old" to "new"`.
#[must_use]
pub fn format_line(
    row: &dyn RecordHistoryFields,
    actor_display: &str,
    actor_href: Option<&str>,
) -> String {
    let _ = actor_href;
    let actor = format!("[{actor_display}]");
    format!("{actor} {}", format_change_line(row))
}
