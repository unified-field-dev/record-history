//! Routes that mount `HistoryTimeline` for Playwright.

use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::components::{Outlet, ParentRoute, Route, Router, Routes};
use leptos_router::path;
use uf_integrations::{
    provide_shell_auth_menu, HostAuthMenu, ShellAppBar, ShellAuthMenu, ShellLeftNav,
    UnifiedFieldAppBar, UnifiedFieldShellLayout,
};
use uf_product::components::{
    Navigation, NavigationBody, NavigationConfig, NavigationLink, NavigationMaterial,
};
use uf_product::{orbital_shell, OrbitalTemplate};

use crate::gate_demos::E2eAuthProvider;
use crate::harness_auth_menu::HarnessAuthMenu;
use crate::pages::{
    HomePage, NotFoundDemoPage, TimelineKindAbsentPage, TimelineKindFilterPage, TimelinePage,
    TimelineRenderersPage,
};

/// SSR document shell.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    orbital_shell(options, || view! { <App/> })
}

/// Root app: product chrome + seeded auth + timeline routes.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    #[cfg(feature = "ssr")]
    {
        provide_context(crate::e2e_higgs_config());
    }

    provide_shell_auth_menu(|| view! { <HarnessAuthMenu /> });

    view! {
        <OrbitalTemplate>
            <E2eAuthProvider>
                <Router>
                    <Routes fallback=|| view! { <NotFoundDemoPage /> }>
                        <ParentRoute path=path!("") view=ChromeShell>
                            <Route path=path!("") view=HomePage />
                            <Route
                                path=path!("history/renderers/:table/:id")
                                view=TimelineRenderersPage
                            />
                            <Route
                                path=path!("history/kind/:table/:id")
                                view=TimelineKindFilterPage
                            />
                            <Route
                                path=path!("history/kind-absent/:table/:id")
                                view=TimelineKindAbsentPage
                            />
                            <Route path=path!("history/:table/:id") view=TimelinePage />
                        </ParentRoute>
                    </Routes>
                </Router>
            </E2eAuthProvider>
        </OrbitalTemplate>
    }
}

#[component]
fn ChromeShell() -> impl IntoView {
    let selected_value = RwSignal::new(None::<String>);
    let open_categories = RwSignal::new(Vec::<String>::new());

    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="History e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ShellLeftNav slot>
                <div data-testid="shell-chrome-left-nav">
                    <Navigation config=NavigationConfig::new().with_selected_value(selected_value).with_open_categories(open_categories)>
                        <NavigationMaterial slot />
                        <NavigationBody slot>
                            <NavigationLink path="/" value="/" icon=icondata::AiHomeOutlined exact=true test_id="nav-home">"Home"</NavigationLink>
                        </NavigationBody>
                    </Navigation>
                </div>
            </ShellLeftNav>
            <Outlet />
        </UnifiedFieldShellLayout>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
    uf_product::hide_boot_loader();
}
