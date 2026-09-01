//! Paginated audit timeline for a parent `HistorySource` record.

use crate::constants::RECORD_HISTORY_PAGE_SIZE;
use crate::get_record_history_page;
use crate::render::map_history_page;
use leptos::prelude::*;
use orbital::primitives::{MessageBar, MessageBarIntent};
use orbital_history::{
    page_fetcher, HistoryPagingMode, HistoryRenderers, HistorySource,
    HistoryTimeline as OrbitalHistoryTimeline,
};
use orbital_macros::component_doc;
use valence::RecordId;

fn is_history_acl_error(err: &ServerFnError) -> bool {
    let msg = err.to_string();
    msg.contains("Not authorized") || msg.contains("Authentication required")
}

/// Self-loading audit timeline for a parent record (`source`).
///
/// Fetches pages via [`get_record_history_page`], maps rows onto Orbital
/// [`HistoryEntry`](orbital_history::HistoryEntry) values, and renders with
/// [`orbital_history::HistoryTimeline`]. Pass `kind_filter` to narrow to specific
/// Valence table names; omit to include all `RecordHistory` implementors for the
/// `source`. Optional [`HistoryRenderers`] register per-kind (or change-region)
/// overrides with fallthrough to Orbital defaults.
///
/// # When to use
///
/// - Detail pages for records that implement `RecordHistory`
/// - Audit trails where newest entries appear first via infinite scroll
///
/// # Usage
///
/// Pass the parent `RecordId` and optionally filter by history table names or
/// supply custom renderers.
///
/// # Examples
///
/// ## Default embed
/// <!-- preview -->
/// ```rust,ignore
/// use record_history_leptos::HistoryTimeline;
/// use valence::RecordId;
/// view! {
///     <div data-testid="history-timeline-preview" style="height: 360px; display: flex; flex-direction: column;">
///         <HistoryTimeline source=RecordId::new("tag", "preview-tag") />
///     </div>
/// }
/// ```
///
/// ## Custom kind renderer
/// <!-- preview -->
/// ```rust,ignore
/// use leptos::prelude::*;
/// use record_history_leptos::{
///     HistoryChange, HistoryEntryView, HistoryKindEntryRow, HistoryRenderContext,
///     HistoryRenderers, HistoryTimeline,
/// };
/// use std::collections::HashMap;
/// use std::sync::Arc;
/// use valence::RecordId;
/// let mut kind_views = HashMap::new();
/// kind_views.insert(
///     "tag_history".into(),
///     Arc::new(|ctx: HistoryRenderContext| {
///         let entry = ctx.entry.clone();
///         let summary = match &entry.change {
///             HistoryChange::FieldDiff { new_value, .. } => new_value.clone(),
///             HistoryChange::Custom { summary } => summary.clone(),
///             _ => String::new(),
///         };
///         Some(view! {
///             <HistoryKindEntryRow entry=entry>
///                 <span data-testid="tag-history-custom-row">{summary}</span>
///             </HistoryKindEntryRow>
///         }.into_any())
///     }) as HistoryEntryView,
/// );
/// let renderers = HistoryRenderers { kind_views, ..Default::default() };
/// view! {
///     <div data-testid="history-timeline-renderers-preview" style="height: 360px; display: flex; flex-direction: column;">
///         <HistoryTimeline
///             source=RecordId::new("tag", "preview-tag")
///             renderers=renderers
///         />
///     </div>
/// }
/// ```
#[component_doc(
    category = "Unified Field",
    preview_slug = "history-timeline",
    preview_label = "History Timeline",
    preview_icon = icondata::AiHistoryOutlined,
    preview = "manual",
)]
#[component]
pub fn HistoryTimeline(
    /// Parent record under audit (tag, transaction, E2E fixture source, …).
    source: RecordId,
    /// Concrete table names to include; empty = all `RecordHistory` implementors.
    #[prop(default = Vec::new())]
    kind_filter: Vec<String>,
    /// Scroll container max height (CSS value). Defaults to `400px`.
    #[prop(optional, into)]
    max_height: Option<String>,
    /// Optional Orbital render overrides (`kind_views` keyed by Valence table name).
    #[prop(optional)]
    renderers: Option<HistoryRenderers>,
) -> impl IntoView {
    let max_h = max_height.unwrap_or_else(|| "400px".into());
    let kinds_arg = if kind_filter.is_empty() {
        None
    } else {
        Some(kind_filter)
    };
    let sid = source;
    let access_denied = RwSignal::new(false);
    let load_failed = RwSignal::new(false);
    let fetcher = page_fetcher(move |page| {
        let sid = sid.clone();
        let kinds_arg = kinds_arg.clone();
        async move {
            match get_record_history_page(page.offset, page.limit, sid, kinds_arg).await {
                Ok(page) => {
                    access_denied.set(false);
                    load_failed.set(false);
                    Ok(map_history_page(page))
                }
                Err(err) => {
                    if is_history_acl_error(&err) {
                        access_denied.set(true);
                        load_failed.set(false);
                    } else {
                        access_denied.set(false);
                        load_failed.set(true);
                    }
                    Err(err)
                }
            }
        }
    });
    let renderers = renderers.unwrap_or_default();

    view! {
        <div
            data-testid="record-history-access-denied"
            hidden=move || !access_denied.get()
        >
            <MessageBar intent=MessageBarIntent::Error>
                "Not authorized to view this history"
            </MessageBar>
        </div>
        <div
            data-testid="record-history-load-failed"
            hidden=move || !load_failed.get()
        >
            <MessageBar intent=MessageBarIntent::Error>
                "Failed to load record history"
            </MessageBar>
        </div>
        <div
            data-testid="record-history-timeline"
            hidden=move || access_denied.get() || load_failed.get()
        >
            <OrbitalHistoryTimeline
                data_source=HistorySource::Server {
                    fetcher,
                    page_size: RECORD_HISTORY_PAGE_SIZE,
                }
                max_height=max_h
                paging=HistoryPagingMode::InfiniteScroll
                renderers=renderers
            />
        </div>
    }
}
