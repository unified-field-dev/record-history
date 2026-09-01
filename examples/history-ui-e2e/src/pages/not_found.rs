//! Not-found demo route.

use leptos::prelude::*;
use uf_integrations::{ShellAuthMenu, UnifiedFieldNotFoundPage};

#[component]
pub fn NotFoundDemoPage() -> impl IntoView {
    view! {
        <UnifiedFieldNotFoundPage>
            <ShellAuthMenu slot:auth_menu>
                <span>"Demo user"</span>
            </ShellAuthMenu>
        </UnifiedFieldNotFoundPage>
    }
}
