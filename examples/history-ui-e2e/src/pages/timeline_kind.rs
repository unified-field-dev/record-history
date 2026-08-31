//! Timeline page that embeds [`HistoryTimeline`] with a `kind_filter`.

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use record_history_leptos::{HistoryTimeline, E2E_RECORD_HISTORY_KIND};
use uf_product::components::ContentContainer;
use valence::RecordId;

/// `/history/kind/:table/:id` — server page narrowed to the fixture history kind.
#[component]
pub fn TimelineKindFilterPage() -> impl IntoView {
    let params = use_params_map();
    let p = params.get();
    let source = RecordId::new(
        p.get("table").unwrap_or_default(),
        p.get("id").unwrap_or_default(),
    );

    view! {
        <ContentContainer data_testid="history-e2e-kind-filter-page">
            <HistoryTimeline
                source=source
                kind_filter=vec![E2E_RECORD_HISTORY_KIND.to_string()]
            />
        </ContentContainer>
    }
}

/// `/history/kind-absent/:table/:id` — filter to a kind with no rows (empty, not deny).
#[component]
pub fn TimelineKindAbsentPage() -> impl IntoView {
    let params = use_params_map();
    let p = params.get();
    let source = RecordId::new(
        p.get("table").unwrap_or_default(),
        p.get("id").unwrap_or_default(),
    );

    view! {
        <ContentContainer data_testid="history-e2e-kind-absent-page">
            <HistoryTimeline
                source=source
                kind_filter=vec!["e2e_record_history_kind_absent".to_string()]
            />
        </ContentContainer>
    }
}
