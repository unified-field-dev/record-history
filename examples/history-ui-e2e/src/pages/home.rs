//! Home page for the History Playwright host.

use leptos::prelude::*;
use uf_product::components::{Body1, ContentContainer, Title3};
use uf_product::primitives::{Flex, FlexGap};

/// Landing page. Timeline pages live at `/history/:table/:id`.
#[component]
pub fn HomePage() -> impl IntoView {
    view! {
        <ContentContainer data_testid="history-e2e-home">
            <Flex vertical=true gap=FlexGap::Medium full_width=true>
                <Title3>"History e2e"</Title3>
                <Body1>
                    "Timelines are at /history/:table/:id. This host seeds auth through POST /api/test/seed-data."
                </Body1>
            </Flex>
        </ContentContainer>
    }
}
