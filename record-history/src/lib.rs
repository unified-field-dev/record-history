//! Shared Valence audit-history traits and read helpers.
//!
//! Products opt in with `traits: [RecordHistory]` on concrete history tables.
//! Opt-in alone does not append audit rows — products write them (usually a
//! [`valence::SideEffect`] on the audited model). Cross-table reads use
//! generated [`RecordHistoryQueryAll`] and [`history_for_source`]. This crate is
//! Valence-only (no Leptos dependency); the timeline UI lives in sibling crate
//! `record-history-leptos` (`HistoryTimeline`).
//!
//! ## Features
//!
//! - **RecordHistory trait schema** — Shared audit field contract (`source`,
//!   `field_name`, `old_value`, `new_value`, `changed_at`, `actor`) so product
//!   history tables reuse one trait instead of copying columns. Parents that can
//!   be audited use `traits: [HistorySource]`. Opt-in enables codegen; it does
//!   not write history by itself.
//!   [Get started](#opt-in-recordhistory-trait)
//! - **Audit row writes** — Append concrete history rows with generated
//!   `Model::create` / `upsert`, typically from a [`valence::SideEffect`] on the
//!   audited parent (Tag's `TagHistoryWriter` is the product pattern). Prefer
//!   [`history_edge_added`], [`history_edge_removed`], and [`history_field_changed`]
//!   so formatters and Orbital FieldDiff stay consistent.
//!   [Get started](#write-audit-history-rows)
//! - **Source-scoped history reads** — [`history_for_source`] authorizes against
//!   the parent row, then loads every `RecordHistory` implementor for that
//!   source as trait models. Prefer it over raw [`RecordHistoryQueryAll`] when
//!   parent ACL matters.
//!   [Get started](#read-history-for-a-source)
//!
//! Format helpers ([`format_line`], [`format_change_line`]), row identity
//! ([`history_row_identity`]), and [`resolve_history_source`] sit beside those
//! capabilities. Sibling `record-history-leptos` maps non-lifecycle rows onto
//! Orbital FieldDiff via `history_row_view_to_entry` (raw `old_value` /
//! `new_value` on the page DTO). Paginated Leptos UI is in `record-history-leptos`.
//!
//! ## Opt in RecordHistory trait
//!
//! The `RecordHistory` Valence trait defines the shared audit field contract
//! products reuse instead of copying column names. Opt a concrete history table
//! (for example `tag_history` or an E2E fixture) into the trait at
//! schema-compile time so codegen emits models, queries, and trait-union
//! accessors. Declare the trait on your history table inside `valence_schema!`
//! before running product codegen. Tables still stay empty until something
//! writes rows — see [Write audit history rows](#write-audit-history-rows).
//!
//! **Prerequisites:** A product (or platform) `valence_schema!` crate that can
//! depend on this crate's trait schemas; parents that own audit rows also declare
//! `traits: [HistorySource]`.
//!
//! ```rust,ignore
//! use record_history::E2eRecordHistoryFixture;
//! use valence::prelude::*;
//!
//! // Product history tables use this opt-in shape (platform fixture shown for the assert):
//! valence_schema! {
//!     TagHistory {
//!         table: "tag_history",
//!         version: "0.1.0",
//!         database: "tag",
//!         description: "Field-level audit for Tag",
//!         traits: [RecordHistory],
//!         fields: [],
//!     }
//! }
//!
//! // Codegen for an opted-in table exposes Model constructors with the shared audit fields.
//! let row = E2eRecordHistoryFixture::new(
//!     valence::RecordId::new("tag", "1"),
//!     "name".to_string(),
//!     "old".to_string(),
//!     "new".to_string(),
//!     chrono::Utc::now(),
//!     None,
//! )?;
//! assert_eq!(row.field_name(), "name");
//! ```
//!
//! On success the table joins [`RecordHistoryQueryAll`] / [`history_for_source`].
//! Schema or codegen failures surface at build time. Constructing a model in
//! memory is not persistence — call create/upsert (or a side effect that does).
//!
//! **Next:** [Write audit history rows](#write-audit-history-rows), then
//! [Read history for a source](#read-history-for-a-source).
//!
//! ## Write audit history rows
//!
//! Opting into `RecordHistory` does not invent writers. Products append rows on
//! their concrete history table with generated `Model::create` / `upsert`, almost
//! always from a [`valence::SideEffect`] registered on the audited parent so
//! every create/update/delete leaves an audit trail. Tag's `TagHistoryWriter`
//! is the live product pattern: on each `Tag` mutation it builds a `TagHistory`
//! row (shared trait fields plus any product extras) and persists under a
//! System actor when create policy is `SYSTEM_ONLY`.
//!
//! **Prerequisites:** History table already declared with
//! `traits: [RecordHistory]`; parent uses `traits: [HistorySource]`; a Valence
//! session allowed to create history rows (often System after capturing the
//! request actor on the row).
//!
//! ```rust,ignore
//! use record_history::E2eRecordHistoryFixture;
//! use valence::{Model, RecordId};
//!
//! // Direct write — same Model::create / upsert a SideEffect would call.
//! // Product code usually wraps this in SideEffect<Parent>::on_mutation.
//! let source = RecordId::new("tag", "tag-1");
//! let row = E2eRecordHistoryFixture::new(
//!     source.clone(),
//!     "name".to_string(),
//!     "Office".to_string(),
//!     "Office Supplies".to_string(),
//!     chrono::Utc::now(),
//!     None, // actor: None => timeline shows "System"
//! )?;
//! E2eRecordHistoryFixture::upsert("row-1", row, &valence).await?;
//!
//! let stored = E2eRecordHistoryFixture::get("row-1", &valence)
//!     .await?
//!     .expect("row written");
//! assert_eq!(stored.field_name(), "name");
//! assert_eq!(stored.source(), &source);
//! ```
//!
//! **Variant — edge / field helpers:** for relation events use
//! [`history_edge_added`] / [`history_edge_removed`] (empty sentinel on the
//! unused side, target id on the other). For scalars use
//! [`history_field_changed`]. [`format_change_line`] turns those into
//! `added …` / `removed …` / `changed …` lines; leptos maps non-lifecycle rows
//! to Orbital FieldDiff with the same raw old/new strings.
//!
//! ```rust,ignore
//! use record_history::{history_edge_added, history_field_changed};
//!
//! let grant = history_edge_added("granted_users", "user:42");
//! assert!(grant.old_value.is_empty());
//! assert_eq!(grant.new_value, "user:42");
//!
//! let rename = history_field_changed("name", "Office", "Office Supplies");
//! assert_eq!(rename.field_name, "name");
//! ```
//!
//! **Variant — SideEffect on the parent:** implement `SideEffect<YourModel>`,
//! map each `MutationKind` to one or more history `create` calls (field diffs on
//! update; `created` / `deleted` lifecycle rows). Keep the request actor on the
//! history row, then switch to `Actor::System` for the write when policy requires
//! it. Extra product columns (for example Tag's `subject_display_name`) live on
//! the concrete schema beside the trait fields.
//!
//! On success the row is visible to [`history_for_source`] and to
//! `record-history-leptos::HistoryTimeline`. Create/upsert failures propagate as
//! `valence::Error` (side effects often log and map to `valence::Error::Internal`).
//!
//! **Next:** [Read history for a source](#read-history-for-a-source), or embed
//! `HistoryTimeline` once rows exist.
//!
//! ## Read history for a source
//!
//! [`history_for_source`] authorizes the request actor against the parent row,
//! then loads every `RecordHistory` implementor registered in the process
//! [`valence::TraitRegistry`] for that source and returns trait models. Call from
//! SSR services, Higgs server functions, or custom handlers — not from raw
//! [`RecordHistoryQueryAll`] when parent ACL matters.
//!
//! **Prerequisites:** Enable the `ssr` feature. The `source` must identify a
//! `HistorySource` parent the actor may read. History tables must already opt in
//! with `traits: [RecordHistory]`, and something must have written rows (see
//! [Write audit history rows](#write-audit-history-rows)).
//!
//! ```rust,ignore
//! use record_history::{history_for_source, E2eRecordHistoryFixture};
//! use valence::{Model, RecordId};
//!
//! // 1. Write — same generated Model path a side effect would call.
//! let row = E2eRecordHistoryFixture::new(
//!     source.clone(),
//!     "name".to_string(),
//!     "Office".to_string(),
//!     "Office Supplies".to_string(),
//!     chrono::Utc::now(),
//!     None,
//! )?;
//! E2eRecordHistoryFixture::upsert("row-1", row, &valence).await?;
//!
//! // 2. Read — ACL-aware load across every RecordHistory implementor for source.
//! let rows = history_for_source(&source, &valence).await?;
//! assert_eq!(rows[0].field_name(), "name");
//! ```
//!
//! On success you hold [`RecordHistoryModel`] rows (newest-first ordering is up
//! to the caller when using raw query builders). Parent denials return
//! [`HistoryError::AccessDenied`] with Display [`HISTORY_ACCESS_DENIED`]. Query
//! failures map to [`HistoryError::Query`]. Format helpers never fail. Sibling
//! `record-history-leptos` maps SSR failures to opaque `ServerFnError`
//! (`Failed to load record history` or the access-denied string).
//!
//! **Variant — raw trait-union query:** [`RecordHistoryQueryAll`] with
//! `where_source` / `where_is_<table>()` when you already hold an authorized
//! Valence session and need narrowing without the helper. Direct union queries
//! may still bypass this helper; prefer `history_for_source` when parent ACL matters. History schemas that set `defer_to_edge` inherit parent Read on entity checks.
//!
//! **Next:** Embed `record-history-leptos::HistoryTimeline`, or run
//! `cargo test -p record-history --test history_api_contract`.
//!
//! ## Examples
//!
//! Start with [Write audit history rows](#write-audit-history-rows), then
//! [Read history for a source](#read-history-for-a-source). Schema opt-in is under
//! [Opt in RecordHistory trait](#opt-in-recordhistory-trait).
//!
//! Run `cargo test -p record-history --test history_api_contract`,
//! `parent_acl_integration`, and the `source_*` suites. Workspace examples:
//! `timeline-host` (Axum oneshot) and `history-ui-e2e` (hydrate + Playwright).
//!
//! ## Feature flags
//!
//! | Flag | What it enables |
//! |------|-----------------|
//! | `ssr` (default) | Valence + SQLite, Lepton SSR, async trait helpers for reads and schemas |

pub mod embedded_surreal;
pub mod error;
pub mod format;
pub mod generated;
pub mod source;
pub mod write;

mod schemas;

pub use error::{HistoryAccessDeniedReason, HistoryError, HISTORY_ACCESS_DENIED};
pub use format::{format_change_line, format_line};
pub use generated::{
    E2eHistorySourceA, E2eHistorySourceB, E2eHistorySourceOwned, E2eRecordHistoryFixture,
    E2eRecordHistoryFixtureAlt, HistorySourceFields, HistorySourceModel, HistorySourceQueryAll,
    HistorySourceQueryRefineE2eHistorySourceA, HistorySourceQueryRefineE2eHistorySourceB,
    HistorySourceQueryRefineE2eHistorySourceOwned, RecordHistoryFields, RecordHistoryModel,
    RecordHistoryQuery, RecordHistoryQueryAll, RecordHistoryQueryRefineE2eRecordHistoryFixture,
    RecordHistoryQueryRefineE2eRecordHistoryFixtureAlt,
};
pub use source::{
    authorize_history_source_read, history_for_source, resolve_history_source,
    ResolvedHistorySource,
};
pub use write::{
    history_created, history_deleted, history_edge_added, history_edge_removed,
    history_field_changed, HistoryWriteParts,
};

/// Force-link schema/trait `inventory` submissions and initialize the global
/// [`valence::TraitRegistry`]. Safe to call more than once.
#[inline(never)]
pub fn touch_schema_inventory() {
    let _ = (
        std::any::type_name::<E2eHistorySourceA>(),
        std::any::type_name::<E2eHistorySourceB>(),
        std::any::type_name::<E2eHistorySourceOwned>(),
        std::any::type_name::<E2eRecordHistoryFixture>(),
        std::any::type_name::<E2eRecordHistoryFixtureAlt>(),
    );
    let _ = valence::TraitRegistry::global();
}

#[inline(never)]
fn ensure_schema_inventory_linked() {
    touch_schema_inventory();
}

#[used]
static __RECORD_HISTORY_SCHEMA_INVENTORY: fn() = ensure_schema_inventory_linked;

/// Concrete table name and bare PK for a history row (`kind`, `row_id`).
#[must_use]
pub fn history_row_identity(id: &valence::RecordId) -> (&str, &str) {
    (id.table(), id.id())
}

#[cfg(test)]
mod tests {
    use super::history_row_identity;

    #[test]
    fn history_row_identity_splits_table_and_id_happy_path() {
        let id = valence::RecordId::new("tag_history", "abc");
        assert_eq!(history_row_identity(&id), ("tag_history", "abc"));

        let other = valence::RecordId::new("e2e_record_history_fixture", "row-1");
        assert_eq!(
            history_row_identity(&other),
            ("e2e_record_history_fixture", "row-1")
        );
    }
}
