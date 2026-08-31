//! Product surface contracts for record-history-leptos (sibling crate).
//!
//! Lives under `record-history` so CI can gate timeline testid / session / DTO
//! needles without compiling Orbital/turf UI when host pins churn. Pattern
//! matches gauge/tag `tests/product_surface.rs` and lepton-uf-app
//! `lepton-shell/tests/product_surface.rs`.

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read_leptos(rel: &str) -> String {
    let path = workspace_root()
        .join("record-history-leptos")
        .join("src")
        .join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn lib_reexports_timeline_and_page_happy_path() {
    let lib = read_leptos("lib.rs");
    for needle in [
        "get_record_history_page",
        "pub use components::HistoryTimeline",
        "HistoryRowView",
        "clamp_history_page_limit",
    ] {
        assert!(
            lib.contains(needle),
            "record-history-leptos lib missing export `{needle}`"
        );
    }
    assert!(
        lib.contains("pub use get_record_history_page::{")
            || lib.contains("pub use get_record_history_page::get_record_history_page"),
        "lib must re-export get_record_history_page"
    );
}

#[test]
fn lib_drop_timeline_export_sad_path() {
    let lib = read_leptos("lib.rs");
    assert!(
        lib.contains("HistoryTimeline"),
        "dropping HistoryTimeline export breaks product embedders"
    );
    assert!(
        lib.contains("get_record_history_page"),
        "dropping get_record_history_page export breaks SSR page fetch"
    );
}

#[test]
fn timeline_testid_and_fetch_happy_path() {
    let timeline = read_leptos("components/history_timeline.rs");
    for needle in [
        "record-history-timeline",
        "record-history-access-denied",
        "get_record_history_page",
        "orbital_history",
        "HistorySource::Server",
        "map_history_page",
        "renderers",
        "RECORD_HISTORY_PAGE_SIZE",
        "is_history_acl_error",
    ] {
        assert!(
            timeline.contains(needle),
            "HistoryTimeline missing contract `{needle}`"
        );
    }
    assert!(
        !timeline.contains("OrbitalInfiniteScroll"),
        "HistoryTimeline must not fork OrbitalInfiniteScroll; use orbital_history::HistoryTimeline"
    );
    assert!(
        !timeline.contains("HistoryTimelineItem"),
        "bespoke HistoryTimelineItem must be removed"
    );
}

#[test]
fn timeline_drop_testid_or_fetch_sad_path() {
    let timeline = read_leptos("components/history_timeline.rs");
    assert!(
        timeline.contains("data-testid=\"record-history-timeline\""),
        "dropping record-history-timeline breaks host / future Playwright parity"
    );
    assert!(
        timeline.contains("get_record_history_page"),
        "timeline must bind get_record_history_page for pagination"
    );
    assert!(
        !timeline.contains("unimplemented!"),
        "HistoryTimeline must not ship unimplemented placeholders"
    );
}

#[test]
fn orbital_bridge_and_mapper_happy_path() {
    let lib = read_leptos("lib.rs");
    assert!(
        lib.contains("history_row_view_to_entry") && lib.contains("HistoryRenderers"),
        "lib must re-export mapper and HistoryRenderers"
    );
    let map = read_leptos("render/map_entry.rs");
    for needle in [
        "history_row_view_to_entry",
        "map_history_page",
        "HistoryChange::Created",
        "HistoryChange::Deleted",
        "HistoryChange::FieldDiff",
        "UNIX_EPOCH",
    ] {
        assert!(
            map.contains(needle),
            "map_entry missing contract `{needle}`"
        );
    }
}

#[test]
fn deleted_row_modules_absent_sad_path() {
    let components = workspace_root().join("record-history-leptos/src/components");
    for dead in [
        "history_timeline_item.rs",
        "history_row_actor.rs",
        "history_row_change.rs",
        "history_row_timestamp.rs",
    ] {
        let path = components.join(dead);
        assert!(
            !path.exists(),
            "dead bespoke row module must be deleted: {}",
            path.display()
        );
    }
    let mod_rs = read_leptos("components/mod.rs");
    assert!(
        !mod_rs.contains("history_timeline_item")
            && !mod_rs.contains("history_row_actor")
            && !mod_rs.contains("history_row_change")
            && !mod_rs.contains("history_row_timestamp"),
        "components/mod.rs must not declare deleted row modules"
    );
}

#[test]
fn page_require_session_happy_path() {
    let page = read_leptos("get_record_history_page.rs");
    assert!(
        page.contains("fn require_session")
            && page.contains("Authentication required")
            && page.contains("session_user_id()"),
        "get_record_history_page must fail closed without a session"
    );
    assert!(
        page.contains("history_for_source") && page.contains("HISTORY_ACCESS_DENIED"),
        "get_record_history_page must authorize via history_for_source after session"
    );
    assert!(
        page.contains("get_record_history_page")
            && page.contains("#[server(GetRecordHistoryPage)]"),
        "page server fn must stay registered as GetRecordHistoryPage"
    );
}

#[test]
fn page_drop_require_session_sad_path() {
    let page = read_leptos("get_record_history_page.rs");
    let start = page
        .find("pub async fn get_record_history_page")
        .expect("get_record_history_page");
    let body = &page[start..];
    assert!(
        body.contains("require_session(&ctx)?"),
        "get_record_history_page must call require_session before valence / query"
    );
    assert!(
        body.contains("history_for_source"),
        "get_record_history_page must call history_for_source after session"
    );
}

#[test]
fn page_clamps_limit_offset_and_kinds_happy_path() {
    let page = read_leptos("get_record_history_page.rs");
    for needle in [
        "fn clamp_history_page_limit",
        "fn clamp_history_page_offset",
        "fn sanitize_kind_filter",
        "MAX_HISTORY_PAGE_LIMIT",
        "clamp_history_page_limit(limit)",
        "clamp_history_page_offset(offset)",
        "sanitize_kind_filter(kinds)",
        "MAX_HISTORY_SCAN",
    ] {
        assert!(
            page.contains(needle),
            "get_record_history_page missing page-arg hardening `{needle}`"
        );
    }
}

#[test]
fn page_drop_limit_clamp_sad_path() {
    let page = read_leptos("get_record_history_page.rs");
    let start = page
        .find("pub async fn get_record_history_page")
        .expect("get_record_history_page");
    let body = &page[start..start + 2200.min(page.len() - start)];
    assert!(
        body.contains("clamp_history_page_limit(limit)")
            && body.contains("clamp_history_page_offset(offset)")
            && body.contains("sanitize_kind_filter(kinds)"),
        "SSR body must clamp limit/offset and sanitize kinds before query"
    );
}

#[test]
fn actor_profile_href_sanitizer_happy_path() {
    let actor = read_leptos("server/actor.rs");
    for needle in [
        "fn actor_profile_href",
        "fn is_safe_user_path_segment",
        "actor_profile_href(&bare_user_id)",
        "://",
    ] {
        assert!(
            actor.contains(needle),
            "actor presentation missing href sanitizer `{needle}`"
        );
    }
}

#[test]
fn actor_profile_href_raw_format_sad_path() {
    let actor = read_leptos("server/actor.rs");
    assert!(
        !actor.contains("let href = Some(format!(\"/user/{bare_user_id}\"))"),
        "actor href must go through actor_profile_href, not raw format!"
    );
    assert!(
        actor.contains("actor_profile_href(&bare_user_id)"),
        "resolve_actor_presentation must call actor_profile_href"
    );
}

#[test]
fn page_sanitized_client_err_happy_path() {
    let page = read_leptos("get_record_history_page.rs");
    assert!(
        page.contains("fn client_err")
            && page.contains("Failed to load record history")
            && page.contains("client_err()"),
        "SSR must map internal failures through client_err"
    );
}

#[test]
fn page_raw_error_leak_sad_path() {
    let page = read_leptos("get_record_history_page.rs");
    assert!(
        !page.contains("map_err(|e| ServerFnError::new(e.to_string()))")
            && !page.contains("ServerFnError::new(format!("),
        "page must not stringify raw Valence/internal errors to the client"
    );
    assert!(
        page.contains("Failed to load record history"),
        "sanitized client message must remain the sole operator-facing failure string"
    );
}

#[test]
fn history_row_view_dto_happy_path() {
    let view = read_leptos("render/row_view.rs");
    for needle in [
        "pub struct HistoryRowView",
        "pub change_line: String",
        "pub actor_label: String",
        "pub kind: String",
        "pub row_id: String",
    ] {
        assert!(
            view.contains(needle),
            "HistoryRowView missing field contract `{needle}`"
        );
    }
    let map = read_leptos("server/map_row.rs");
    assert!(
        map.contains("format_change_line") && map.contains("change_line"),
        "into_history_row_view must format change_line from the domain helper"
    );
}

#[test]
fn history_row_view_includes_raw_values_for_renderers() {
    let view = read_leptos("render/row_view.rs");
    let struct_start = view
        .find("pub struct HistoryRowView")
        .expect("HistoryRowView");
    let body = &view[struct_start..];
    assert!(
        body.contains("pub old_value: String") && body.contains("pub new_value: String"),
        "HistoryRowView must ship old_value/new_value for custom renderers"
    );
    assert!(
        body.contains("pub change_line: String"),
        "DTO must keep change_line as the default presentation field"
    );
}

#[test]
fn actor_presentation_hygiene_happy_path() {
    let actor = read_leptos("server/actor.rs");
    for needle in [
        "resolve_actor_presentation",
        "\"System\"",
        "opaque_actor_label",
        "never email",
        "never System",
    ] {
        assert!(
            actor.contains(needle),
            "actor presentation missing contract `{needle}`"
        );
    }
}

#[test]
fn actor_email_leak_sad_path() {
    let actor = read_leptos("server/actor.rs");
    assert!(
        !actor.contains(".email") && !actor.contains("user.email") && !actor.contains("email()"),
        "actor label path must not format user email onto the timeline"
    );
    assert!(
        actor.contains("opaque_actor_label") && actor.contains("\"System\""),
        "fallback labels must stay opaque User / System"
    );
}

#[test]
fn timeline_host_inventory_sync_happy_path() {
    let host = fs::read_to_string(workspace_root().join("examples/timeline-host/src/main.rs"))
        .expect("timeline-host main.rs");
    for needle in [
        "\"component\": \"HistoryTimeline\"",
        "\"testid\": \"record-history-timeline\"",
        "\"auth_gate\": \"require_session\"",
        "\"server_fn\": \"GetRecordHistoryPage\"",
        "history_for_source",
        "format_line",
        "sync_typed_tables_from_registry",
        "FIXTURE_ACTOR_ID",
    ] {
        assert!(
            host.contains(needle),
            "timeline-host missing contract `{needle}`"
        );
    }
    let timeline = read_leptos("components/history_timeline.rs");
    assert!(
        timeline.contains("data-testid=\"record-history-timeline\"")
            && timeline.contains("HistoryTimeline"),
        "host inventory must stay aligned with HistoryTimeline"
    );
    let page = read_leptos("get_record_history_page.rs");
    assert!(
        page.contains("fn require_session") && page.contains("#[server(GetRecordHistoryPage)]"),
        "host auth_gate / server_fn must stay aligned with get_record_history_page"
    );
}
