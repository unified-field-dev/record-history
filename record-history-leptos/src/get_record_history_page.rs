use crate::render::HistoryRowView;
use leptos::prelude::*;
use orbital_paging::Page;
use valence::RecordId;

#[cfg(feature = "ssr")]
use record_history::RecordHistoryModel;

/// Maximum page size accepted by [`get_record_history_page`].
///
/// Caps resource abuse from forged `limit` while remaining above
/// [`crate::RECORD_HISTORY_PAGE_SIZE`] (25).
pub const MAX_HISTORY_PAGE_LIMIT: u32 = 50;

/// Maximum zero-based `offset` accepted by [`get_record_history_page`].
pub const MAX_HISTORY_PAGE_OFFSET: u32 = 10_000;

/// Hard cap on rows scanned when counting or post-filtering by `kinds`.
#[cfg(feature = "ssr")]
const MAX_HISTORY_SCAN: u32 = 10_000;

/// Maximum number of kind-filter table names accepted from the client.
const MAX_KIND_FILTER_LEN: usize = 32;

/// Maximum characters per kind-filter table name.
const MAX_KIND_NAME_CHARS: usize = 128;

/// Clamp a client-supplied page `limit` into `1..=MAX_HISTORY_PAGE_LIMIT`.
#[must_use]
pub fn clamp_history_page_limit(limit: u32) -> u32 {
    limit.clamp(1, MAX_HISTORY_PAGE_LIMIT)
}

/// Clamp a client-supplied page `offset` into `0..=MAX_HISTORY_PAGE_OFFSET`.
#[must_use]
pub fn clamp_history_page_offset(offset: u32) -> u32 {
    offset.min(MAX_HISTORY_PAGE_OFFSET)
}

/// Drop empty names, truncate overlong names, and cap filter cardinality.
#[must_use]
pub fn sanitize_kind_filter(kinds: Option<Vec<String>>) -> Option<Vec<String>> {
    let kinds = kinds?;
    let cleaned: Vec<String> = kinds
        .into_iter()
        .filter(|k| !k.is_empty())
        .take(MAX_KIND_FILTER_LEN)
        .map(|k| {
            if k.chars().count() <= MAX_KIND_NAME_CHARS {
                k
            } else {
                k.chars().take(MAX_KIND_NAME_CHARS).collect()
            }
        })
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Paginated history rows for [`crate::HistoryTimeline`] (newest first).
///
/// Always `async` because the `#[server]` macro requires it; the SSR body
/// awaits, while the non-SSR stub returns immediately.
///
/// ## Errors
///
/// Returns `ServerFnError` for auth failures, Valence/query failures, and
/// actor-resolution failures. Client-visible Valence details are collapsed to
/// `Failed to load record history`.
///
/// ## Security
///
/// Requires an authenticated session **and** Read authorization on the source
/// parent record ([`record_history::history_for_source`]). Session presence alone
/// is not enough. Missing, unsupported, and parent-read denials return
/// [`record_history::HISTORY_ACCESS_DENIED`]. Client `limit` / `offset` /
/// `kinds` are clamped before paging.
#[server(GetRecordHistoryPage)]
pub async fn get_record_history_page(
    /// Zero-based index of the first history row to return.
    offset: u32,
    /// Maximum number of history rows to return.
    limit: u32,
    /// Record whose history should be listed.
    source: RecordId,
    /// Optional set of history entry kinds to restrict results to.
    kinds: Option<Vec<String>>,
) -> Result<Page<HistoryRowView>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use crate::server::into_history_row_view;
        use record_history::RecordHistoryFields;

        let ctx = higgs::Higgs::from_request().await?;
        require_session(&ctx)?;

        #[cfg(feature = "e2e-harness")]
        if crate::e2e_harness::e2e_history_load_fault_active() {
            return Err(client_err());
        }

        let v = ctx.valence().map_err(|e| {
            tracing::error!(error = %e, operation = "get_record_history_page", "valence from request");
            client_err()
        })?;

        tracing::debug!(
            operation = "get_record_history_page",
            table = source.table(),
            record_id = source.id(),
            "paging history"
        );

        // Refined per-table queries via history_for_source (parent ACL first).
        // Trait-union projection misses datetime_unix on mem/SQLite.
        let mut rows = match record_history::history_for_source(&source, &v).await {
            Ok(rows) => rows,
            Err(record_history::HistoryError::AccessDenied { .. }) => {
                return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    operation = "get_record_history_page",
                    "history_for_source"
                );
                return Err(client_err());
            }
        };

        let limit = clamp_history_page_limit(limit);
        let offset = clamp_history_page_offset(offset);
        let kinds = sanitize_kind_filter(kinds);

        rows.sort_by(|a, b| b.changed_at().cmp(a.changed_at()));
        if let Some(kinds) = kinds.as_deref() {
            rows.retain(|r| row_kind_matches(r, kinds));
        }
        if rows.len() > MAX_HISTORY_SCAN as usize {
            rows.truncate(MAX_HISTORY_SCAN as usize);
        }

        let total_rows = rows.len() as u64;
        let start = offset as usize;
        let fetch_n = (limit as usize).saturating_add(1);
        let page_rows: Vec<_> = rows.into_iter().skip(start).take(fetch_n).collect();
        let db_rows_fetched = page_rows.len() as u32;

        let mut views = Vec::with_capacity(page_rows.len());
        for model in page_rows {
            views.push(into_history_row_view(model, &v).await.map_err(|e| {
                tracing::error!(
                    error = %e,
                    operation = "get_record_history_page",
                    "map history row"
                );
                client_err()
            })?);
        }

        let limit_usize = limit as usize;
        if views.len() > limit_usize {
            views.truncate(limit_usize);
        }

        let items_returned = views.len() as u32;
        let next_request_offset = offset.saturating_add(db_rows_fetched);
        let has_more =
            db_rows_fetched > limit || offset.saturating_add(items_returned) < total_rows as u32;

        Ok(Page {
            items: views,
            has_more,
            total_count: Some(total_rows),
            next_request_offset: Some(next_request_offset),
        })
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds);
        Err(ServerFnError::ServerError(
            "GetRecordHistoryPage requires SSR".into(),
        ))
    }
}

#[cfg(feature = "ssr")]
fn require_session(ctx: &higgs::Higgs) -> Result<(), ServerFnError> {
    if ctx.session_user_id().is_some() {
        Ok(())
    } else {
        Err(ServerFnError::new("Authentication required"))
    }
}

#[cfg(feature = "ssr")]
fn row_kind_matches(row: &RecordHistoryModel, kinds: &[String]) -> bool {
    let Some(id) = row.id.as_ref() else {
        return false;
    };
    let (kind, _) = record_history::history_row_identity(id);
    kinds.iter().any(|k| k == kind)
}

/// Sanitize internal errors so clients do not see Valence/table details.
#[cfg(feature = "ssr")]
fn client_err() -> ServerFnError {
    ServerFnError::new("Failed to load record history")
}

#[cfg(test)]
mod tests {
    use super::{
        clamp_history_page_limit, clamp_history_page_offset, sanitize_kind_filter,
        MAX_HISTORY_PAGE_LIMIT, MAX_HISTORY_PAGE_OFFSET,
    };

    #[test]
    fn clamp_history_page_limit_caps_high_values_sad() {
        assert_eq!(clamp_history_page_limit(0), 1);
        assert_eq!(clamp_history_page_limit(25), 25);
        assert_eq!(
            clamp_history_page_limit(MAX_HISTORY_PAGE_LIMIT),
            MAX_HISTORY_PAGE_LIMIT
        );
        assert_eq!(
            clamp_history_page_limit(MAX_HISTORY_PAGE_LIMIT + 500),
            MAX_HISTORY_PAGE_LIMIT
        );
        assert_eq!(clamp_history_page_limit(u32::MAX), MAX_HISTORY_PAGE_LIMIT);
    }

    #[test]
    fn clamp_history_page_offset_caps_high_values_sad() {
        assert_eq!(clamp_history_page_offset(0), 0);
        assert_eq!(clamp_history_page_offset(100), 100);
        assert_eq!(
            clamp_history_page_offset(MAX_HISTORY_PAGE_OFFSET),
            MAX_HISTORY_PAGE_OFFSET
        );
        assert_eq!(
            clamp_history_page_offset(MAX_HISTORY_PAGE_OFFSET + 1),
            MAX_HISTORY_PAGE_OFFSET
        );
        assert_eq!(clamp_history_page_offset(u32::MAX), MAX_HISTORY_PAGE_OFFSET);
    }

    #[test]
    fn sanitize_kind_filter_drops_empty_and_caps_cardinality_sad() {
        assert_eq!(sanitize_kind_filter(None), None);
        assert_eq!(sanitize_kind_filter(Some(vec![String::new()])), None);
        assert_eq!(
            sanitize_kind_filter(Some(vec!["tag_history".into()])),
            Some(vec!["tag_history".into()])
        );

        let many: Vec<String> = (0..64).map(|i| format!("kind_{i}")).collect();
        let cleaned = sanitize_kind_filter(Some(many)).expect("kinds");
        assert_eq!(cleaned.len(), 32);

        let long = "k".repeat(200);
        let cleaned = sanitize_kind_filter(Some(vec![long])).expect("kinds");
        assert_eq!(cleaned[0].chars().count(), 128);
    }
}
