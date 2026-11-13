// The `#[server]` macro expands `get_record_history_page` into a single
// `async fn` shared by both cfg branches: the `ssr` body awaits real I/O,
// but the non-`ssr` stub (used by `hydrate`-only client builds) returns
// immediately. Clippy's `unused_async_trait_impl` fires on the macro's
// generated trait impl, not the annotated item, so a per-item `#[allow]`
// does not reach it — the allow must be file-scoped. Splitting into two
// functions would break the `#[server]` macro's single-signature contract.
#![allow(clippy::unused_async_trait_impl)]

//! Paged history rows for [`crate::HistoryTimeline`], newest first.
//!
//! [`get_record_history_page`] pages by offset only (its signature predates
//! cursors). [`get_record_history_page_by_cursor`] adds `cursor`:
//!
//! - **Cursor path:** a `cursor` from the previous page's `next_cursor` that
//!   decodes for this record, kind filter and page size continues after that
//!   page's last entry (a `record_history::HistoryKey`). `offset` is ignored, so
//!   [`MAX_HISTORY_PAGE_OFFSET`] does not limit how far back a reader can go.
//! - **Offset path:** no cursor, or a stale, foreign or malformed one. `offset`
//!   is clamped to [`MAX_HISTORY_PAGE_OFFSET`] and the newest `offset` entries
//!   are skipped.
//!
//! Both paths return `next_cursor` when `has_more`. `total_count` is set only
//! on the last page, where the count is known without another read.

use crate::render::HistoryRowView;
use leptos::prelude::*;
use orbital_paging::Page;
use valence::RecordId;

/// Maximum page size accepted by [`get_record_history_page`].
///
/// Caps resource abuse from forged `limit` while remaining above
/// [`crate::RECORD_HISTORY_PAGE_SIZE`] (25).
pub const MAX_HISTORY_PAGE_LIMIT: u32 = 50;

/// Maximum zero-based `offset` accepted by [`get_record_history_page`] on the offset path.
pub const MAX_HISTORY_PAGE_OFFSET: u32 = 10_000;

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

/// One page of a record's history, newest first, by `offset`.
///
/// Always `async` because the `#[server]` macro requires it; the SSR body
/// awaits, while the non-SSR stub returns immediately. Takes the offset path
/// only; the returned page still carries `next_cursor`, which
/// [`get_record_history_page_by_cursor`] accepts for the next page.
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
/// parent record ([`record_history::history_for_source_page`]). Session presence
/// alone is not enough. Missing, unsupported, and parent-read denials return
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
        serve_history_page(offset, limit, source, kinds, None).await
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds);
        Err(ServerFnError::ServerError(
            "GetRecordHistoryPage requires SSR".into(),
        ))
    }
}

/// Session check, argument clamps and request actor for both server fns.
#[cfg(feature = "ssr")]
async fn serve_history_page(
    offset: u32,
    limit: u32,
    source: RecordId,
    kinds: Option<Vec<String>>,
    cursor: Option<String>,
) -> Result<Page<HistoryRowView>, ServerFnError> {
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx)?;

    #[cfg(feature = "e2e-harness")]
    if crate::e2e_harness::e2e_history_load_fault_active() {
        return Err(client_err());
    }

    let limit = clamp_history_page_limit(limit);
    let offset = clamp_history_page_offset(offset);
    let kinds = sanitize_kind_filter(kinds);

    let v = ctx.valence().map_err(|e| {
        tracing::error!(error = %e, operation = "get_record_history_page", "valence from request");
        client_err()
    })?;

    load_history_page(&source, &v, offset, limit, kinds.as_deref(), cursor).await
}

/// One page of a record's history, newest first, continued by `cursor`.
///
/// Same as [`get_record_history_page`] plus `cursor`: pass the previous page's
/// `next_cursor` to continue after it, past [`MAX_HISTORY_PAGE_OFFSET`]. A
/// missing, stale or malformed cursor pages by `offset`. [`crate::HistoryTimeline`]
/// loads its pages through this function.
///
/// ## Errors
///
/// As [`get_record_history_page`].
///
/// ## Security
///
/// As [`get_record_history_page`]; a cursor grants nothing.
#[server(GetRecordHistoryPageByCursor)]
pub async fn get_record_history_page_by_cursor(
    /// Zero-based index of the first history row to return (offset path only).
    offset: u32,
    /// Maximum number of history rows to return.
    limit: u32,
    /// Record whose history should be listed.
    source: RecordId,
    /// Optional set of history entry kinds to restrict results to.
    kinds: Option<Vec<String>>,
    /// The previous page's `next_cursor`; `None` pages by `offset`.
    cursor: Option<String>,
) -> Result<Page<HistoryRowView>, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        serve_history_page(offset, limit, source, kinds, cursor).await
    }

    #[cfg(not(feature = "ssr"))]
    {
        let _ = (offset, limit, source, kinds, cursor);
        Err(ServerFnError::ServerError(
            "GetRecordHistoryPageByCursor requires SSR".into(),
        ))
    }
}

/// The body of [`get_record_history_page`] and [`get_record_history_page_by_cursor`]
/// after session and argument checks.
///
/// `offset`, `limit` and `kinds` must already be clamped with
/// [`clamp_history_page_offset`], [`clamp_history_page_limit`] and
/// [`sanitize_kind_filter`]. `valence` carries the request actor; parent Read
/// is checked here through [`record_history::history_for_source_page`].
///
/// ## Errors
///
/// [`record_history::HISTORY_ACCESS_DENIED`] when the parent cannot be read;
/// `Failed to load record history` for any other failure.
#[cfg(feature = "ssr")]
pub async fn load_history_page(
    source: &RecordId,
    v: &valence::Valence,
    offset: u32,
    limit: u32,
    kinds: Option<&[String]>,
    cursor: Option<String>,
) -> Result<Page<HistoryRowView>, ServerFnError> {
    use crate::server::into_history_row_view;
    use orbital_paging::{PageCursor, PageRequest};

    tracing::debug!(
        operation = "get_record_history_page",
        table = source.table(),
        record_id = source.id(),
        cursor = cursor.is_some(),
        "paging history"
    );

    let request = PageRequest::new(offset, limit).with_cursor(cursor);
    let scope = history_cursor_scope(source, kinds);
    let resumed = PageCursor::from_request(&request, &scope).and_then(|c| {
        Some((
            c.position,
            record_history::HistoryKey::parse(c.key.as_deref()?)?,
        ))
    });
    // Rows already shown before this page: the cursor's count, or the offset.
    let (before, after, skip) = match &resumed {
        Some((position, key)) => (*position, Some(key), 0),
        None => (u64::from(offset), None, offset),
    };

    let page =
        match record_history::history_for_source_page(source, v, after, skip, limit, kinds).await {
            Ok(page) => page,
            Err(record_history::HistoryError::AccessDenied { .. }) => {
                return Err(ServerFnError::new(record_history::HISTORY_ACCESS_DENIED));
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    operation = "get_record_history_page",
                    "history_for_source_page"
                );
                return Err(client_err());
            }
        };

    let mut items = Vec::with_capacity(page.rows.len());
    for model in page.rows {
        items.push(into_history_row_view(model, v).await.map_err(|e| {
            tracing::error!(
                error = %e,
                operation = "get_record_history_page",
                "map history row"
            );
            client_err()
        })?);
    }

    let shown = before.saturating_add(items.len() as u64);
    let next_cursor = page.next_key.map(|key| {
        PageCursor::with_key(shown, key.encode(), request.query_fingerprint(&scope)).encode()
    });
    Ok(Page {
        items,
        has_more: page.has_more,
        total_count: (!page.has_more).then_some(shown),
        next_request_offset: Some(u32::try_from(shown).unwrap_or(u32::MAX)),
        ..Page::default()
    }
    .with_next_cursor(next_cursor))
}

/// Cursor fingerprint scope: the record and the kind filter.
#[cfg(feature = "ssr")]
fn history_cursor_scope(source: &RecordId, kinds: Option<&[String]>) -> String {
    format!(
        "record_history:{}:{}|{}",
        source.table(),
        source.id(),
        kinds.map(|k| k.join(",")).unwrap_or_default()
    )
}

#[cfg(feature = "ssr")]
fn require_session(ctx: &higgs::Higgs) -> Result<(), ServerFnError> {
    if ctx.session_user_id().is_some() {
        Ok(())
    } else {
        Err(ServerFnError::new(
            crate::constants::HISTORY_AUTH_REQUIRED_MSG,
        ))
    }
}

/// Sanitize internal errors so clients do not see Valence/table details.
#[cfg(feature = "ssr")]
pub(crate) fn client_err() -> ServerFnError {
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
