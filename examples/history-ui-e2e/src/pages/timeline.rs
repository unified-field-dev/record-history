//! Timeline page that embeds [`HistoryTimeline`].

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use record_history_leptos::HistoryTimeline;
use uf_product::components::ContentContainer;
use valence::RecordId;

/// `/history/:table/:id` — paginated audit reader for one parent record.
#[component]
pub fn TimelinePage() -> impl IntoView {
    let params = use_params_map();
    let p = params.get();
    let source = RecordId::new(
        p.get("table").unwrap_or_default(),
        p.get("id").unwrap_or_default(),
    );

    view! {
        <ContentContainer data_testid="history-e2e-timeline-page">
            <HistoryTimeline source=source />
        </ContentContainer>
    }
}
