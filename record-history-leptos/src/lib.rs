//! Record History timeline UI for Leptos apps.
//!
//! Embed [`HistoryTimeline`] on detail pages to show a paginated audit trail for a
//! parent record. Rows load through [`get_record_history_page`] (session-gated) and
//! render with Orbital timeline chrome. Register per-kind overrides with
//! [`HistoryRenderers`] when a product needs custom row bodies — see
//! [Register history renderers](#register-history-renderers).
//!
//! ## Features
//!
//! - **History timeline component** — Self-loading infinite-scroll audit trail for
//!   a parent history source. Pass `source` (and optional `kind_filter`) on detail
//!   pages; the component calls [`get_record_history_page`], maps rows onto Orbital
//!   entries, and paints stock created / deleted / change-line chrome.
//!   [Get started](#embed-historytimeline)
//! - **Paginated history server fn** — [`get_record_history_page`] returns a
//!   clamped page of [`HistoryRowView`] rows (newest first) for callers that need
//!   the DTO without mounting the timeline.
//!   [Get started](#paginated-ssr-fetch)
//! - **History renderers** — Per-kind (and optional change-region) overrides via
//!   Orbital [`HistoryRenderers`], keyed by Valence history table name, with
//!   fallthrough to stock chrome for every other kind.
//!   [Get started](#register-history-renderers)
//!
//! E2E fixture constants ([`mod@constants`]) support those capabilities. Domain
//! trait schemas, writers, and ACL-aware reads live in `record-history`.
//!
//! ## Embed HistoryTimeline
//!
//! [`HistoryTimeline`] is the product embed for audit trails on detail pages. It
//! loads pages through [`get_record_history_page`], maps each [`HistoryRowView`]
//! onto an Orbital entry, and renders with Orbital's timeline (infinite scroll).
//! Prefer this over product-local `list_*_history` server functions that reimplement
//! the same union query. Mount it when the request actor can read the parent record.
//!
//! **Prerequisites:** Depend on `record-history-leptos` with `ssr` (or `hydrate` for
//! the client half). History tables must opt in with `traits: [RecordHistory]` in
//! `record-history`, and something must write rows (product `SideEffect` or
//! direct create). Server fetch requires an authenticated session — see root
//! `SECURITY.md`.
//!
//! ```rust,ignore
//! use record_history_leptos::HistoryTimeline;
//! use valence::RecordId;
//!
//! let tag_id = "tag-1";
//! view! {
//!     <HistoryTimeline source=RecordId::new("tag", tag_id) />
//!     <HistoryTimeline
//!         source=RecordId::new("tag", tag_id)
//!         kind_filter=vec!["tag_history".into()]
//!     />
//! }
//! assert!(!tag_id.is_empty());
//! ```
//!
//! On success the timeline renders newest-first stock rows (empty state when
//! none). Non-lifecycle rows map to Orbital [`HistoryChange::FieldDiff`] with
//! raw `old_value` / `new_value` (plus a preformatted `change_line` for default
//! chrome). Product `HistoryRenderers` can replace the change body using those
//! strings — for example Avatar rows for relation field names. Load failures
//! surface as the client-visible `Failed to load record history` string from
//! [`get_record_history_page`]. Auth failures use `Authentication required`.
//!
//! **Next:** [Register history renderers](#register-history-renderers) when a kind
//! needs custom chrome, or run workspace example `timeline-host`.
//!
//! ## Paginated SSR fetch
//!
//! [`get_record_history_page`] is the SSR page fetch behind [`HistoryTimeline`].
//! Call it inside `#[server]` / Higgs request handlers when the request needs a
//! clamped page of [`HistoryRowView`] without mounting the timeline. Client
//! `limit` / `offset` / `kinds` are sanitized before query.
//!
//! **Prerequisites:** `ssr` feature; Higgs request context with an authenticated
//! session; a `source` [`RecordId`] the actor may read.
//!
//! ```rust,ignore
//! use record_history_leptos::{get_record_history_page, RECORD_HISTORY_PAGE_SIZE};
//! use valence::RecordId;
//!
//! let page = get_record_history_page(
//!     0,
//!     RECORD_HISTORY_PAGE_SIZE,
//!     RecordId::new("tag", tag_id),
//!     Some(vec!["tag_history".into()]),
//! )
//! .await?;
//! assert!(!page.items.is_empty() || !page.has_more);
//! ```
//!
//! On success you hold an [`orbital_paging::Page`] of [`HistoryRowView`]. Valence
//! / actor-resolution failures stay off the wire as
//! `Failed to load record history`. Auth failures use `Authentication required`.
//! Domain reads in `record-history` keep typed `valence::Error`.
//!
//! **Next:** Prefer [`HistoryTimeline`] when stock Orbital chrome is enough, or
//! [Register history renderers](#register-history-renderers) for per-kind bodies.
//!
//! ## Register history renderers
//!
//! Products that need a different body for one Valence history table (for example
//! `tag_history`) register Orbital [`HistoryRenderers`] on [`HistoryTimeline`].
//! Keys in `kind_views` are table names; return `None` from a view to fall through
//! to stock chrome. Prefer [`HistoryKindEntryRow`] when you want default timestamp
//! and actor chrome with a custom change body. Unregistered kinds keep the default
//! created / deleted / change-line layout.
//!
//! **Prerequisites:** Same `ssr` / `hydrate` and session setup as
//! [Embed HistoryTimeline](#embed-historytimeline). Renderer callbacks run on the
//! client against already-fetched entries — they do not widen the page DTO.
//!
//! ```rust,ignore
//! use leptos::prelude::*;
//! use record_history_leptos::{
//!     HistoryChange, HistoryEntryView, HistoryKindEntryRow, HistoryRenderContext,
//!     HistoryRenderers, HistoryTimeline,
//! };
//! use std::collections::HashMap;
//! use std::sync::Arc;
//! use valence::RecordId;
//!
//! let mut kind_views = HashMap::new();
//! kind_views.insert(
//!     "tag_history".into(),
//!     Arc::new(|ctx: HistoryRenderContext| {
//!         let entry = ctx.entry.clone();
//!         let summary = match &entry.change {
//!             HistoryChange::FieldDiff { new_value, .. } => new_value.clone(),
//!             HistoryChange::Custom { summary } => summary.clone(),
//!             _ => String::new(),
//!         };
//!         Some(view! {
//!             <HistoryKindEntryRow entry=entry>
//!                 <span data-testid="tag-history-custom-row">{summary}</span>
//!             </HistoryKindEntryRow>
//!         }.into_any())
//!     }) as HistoryEntryView,
//! );
//! let renderers = HistoryRenderers { kind_views, ..Default::default() };
//! let tag_id = "tag-1";
//! view! {
//!     <HistoryTimeline
//!         source=RecordId::new("tag", tag_id)
//!         renderers=renderers
//!     />
//! }
//! assert_eq!(tag_id, "tag-1");
//! ```
//!
//! On success, `tag_history` rows use your body; other kinds keep Orbital defaults.
//! For a fully custom list layout (not Orbital timeline chrome), call
//! [`get_record_history_page`] and render [`HistoryRowView`] yourself — see
//! [Paginated SSR fetch](#paginated-ssr-fetch). Deeper Orbital renderer concepts
//! (`entry_view`, `change_view`, fallthrough order) live in `orbital-history`.
//!
//! **Next:** [Embed HistoryTimeline](#embed-historytimeline) for the stock path, or
//! domain helpers in `record-history` (`history_row_identity`, formatters).
//!
//! ## Examples
//!
//! Start with [Embed HistoryTimeline](#embed-historytimeline). Per-kind chrome:
//! [Register history renderers](#register-history-renderers). Page DTO alone:
//! [Paginated SSR fetch](#paginated-ssr-fetch). Domain trait queries, writers, and
//! format helpers live in `record-history`.
//!
//! Workspace example `timeline-host` shows the embed inventory (see its README).
//!
//! ## Feature flags
//!
//! | Flag | What it enables |
//! |------|-----------------|
//! | `ssr` | Leptos SSR, `record-history/ssr`, Higgs, `orbital-history/ssr` |
//! | `hydrate` | Client/WASM half of [`HistoryTimeline`] |
//! | `preview` | Orbital preview catalog constants for [`HistoryTimeline`] |
//! | `e2e-harness` | Process-local load-fault flag for `history-ui-e2e` only |

pub mod constants;
pub mod render;

mod get_record_history_page;

#[cfg(feature = "e2e-harness")]
pub mod e2e_harness;

#[cfg(any(feature = "hydrate", feature = "ssr"))]
pub mod components;

#[cfg(feature = "ssr")]
pub mod server;

// Keep crc32fast 1.5.0 in the graph for leptos-lints (see workspace pin).
use crc32fast as _;

pub use constants::{
    e2e_record_history_empty_source, e2e_record_history_source, E2E_RECORD_HISTORY_EMPTY_SOURCE_ID,
    E2E_RECORD_HISTORY_KIND, E2E_RECORD_HISTORY_ROW_COUNT, E2E_RECORD_HISTORY_SOURCE_ID,
    RECORD_HISTORY_PAGE_SIZE,
};
pub use get_record_history_page::{
    clamp_history_page_limit, clamp_history_page_offset, get_record_history_page,
    sanitize_kind_filter, MAX_HISTORY_PAGE_LIMIT, MAX_HISTORY_PAGE_OFFSET,
};

#[cfg(feature = "e2e-harness")]
pub use e2e_harness::{e2e_history_load_fault_active, set_e2e_history_load_fault};
pub use render::{history_row_view_to_entry, map_history_page, HistoryRowView};

#[cfg(any(feature = "hydrate", feature = "ssr"))]
pub use components::HistoryTimeline;

#[cfg(any(feature = "hydrate", feature = "ssr"))]
pub use orbital_history::{
    HistoryActor, HistoryChange, HistoryChangeView, HistoryEntry, HistoryEntryView,
    HistoryKindEntryRow, HistoryRenderContext, HistoryRenderers,
};

#[cfg(all(any(feature = "hydrate", feature = "ssr"), feature = "preview"))]
pub use components::{HISTORYTIMELINE_DOC, HISTORYTIMELINE_PROPS};

#[cfg(feature = "preview")]
pub mod preview;
