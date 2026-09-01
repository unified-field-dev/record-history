//! Timeline page that embeds [`HistoryTimeline`] with custom `kind_views`.

use std::collections::HashMap;
use std::sync::Arc;

use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use record_history_leptos::{
    HistoryChange, HistoryEntryView, HistoryKindEntryRow, HistoryRenderContext, HistoryRenderers,
    HistoryTimeline, E2E_RECORD_HISTORY_KIND,
};
use uf_product::components::{Body1, Body1Strong, ContentContainer};
use valence::RecordId;

fn fixture_kind_renderers() -> HistoryRenderers {
    let mut kind_views = HashMap::new();
    kind_views.insert(
        E2E_RECORD_HISTORY_KIND.into(),
        Arc::new(|ctx: HistoryRenderContext| {
            let entry = ctx.entry.clone();
            let summary = match &entry.change {
                HistoryChange::FieldDiff { new_value, .. } => new_value.clone(),
                HistoryChange::Custom { summary } => summary.clone(),
                _ => String::new(),
            };
            Some(
                view! {
                    <HistoryKindEntryRow entry=entry>
                        <div data-testid="e2e-fixture-custom-row">
                            <Body1 class="orbital-history__change".to_string()>
                                <Body1Strong>"Custom renderer"</Body1Strong>
                                " — "
                                {summary}
                            </Body1>
                        </div>
                    </HistoryKindEntryRow>
                }
                .into_any(),
            )
        }) as HistoryEntryView,
    );
    HistoryRenderers {
        kind_views,
        ..Default::default()
    }
}

/// `/history/renderers/:table/:id` — custom kind row + fallthrough for other kinds.
#[component]
pub fn TimelineRenderersPage() -> impl IntoView {
    let params = use_params_map();
    let p = params.get();
    let source = RecordId::new(
        p.get("table").unwrap_or_default(),
        p.get("id").unwrap_or_default(),
    );

    view! {
        <ContentContainer data_testid="history-e2e-renderers-page">
            <HistoryTimeline source=source renderers=fixture_kind_renderers() />
        </ContentContainer>
    }
}
