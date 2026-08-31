//! Demo auth menu that binds the shell [`AuthDialogController`].

use leptos::prelude::*;
use uf_product::primitives::{Body1, Button, ButtonAppearance, Title3};
use uf_product::{
    provide_auth_dialog_controller, use_auth_dialog_controller, AuthDialogController,
    AuthDialogIntent,
};

/// App-bar stub plus a harness dialog driven by the shared auth controller.
#[component]
pub fn HarnessAuthMenu() -> impl IntoView {
    let controller = use_auth_dialog_controller().unwrap_or_else(provide_auth_dialog_controller);
    view! {
        <span data-testid="demo-auth-menu">"Demo user"</span>
        <HarnessAuthDialog controller />
    }
}

#[component]
fn HarnessAuthDialog(controller: AuthDialogController) -> impl IntoView {
    let open = controller.open();
    let intent = controller.intent();
    let close = Callback::new(move |_| {
        controller.close();
    });

    view! {
        {move || {
            if !open.get() {
                return ().into_any();
            }
            view! {
                <div data-testid="auth-dialog-root" role="dialog" aria-modal="true">
                    <Title3>
                        {move || match intent.get() {
                            AuthDialogIntent::Signin => "Harness sign in",
                            AuthDialogIntent::Signup => "Harness sign up",
                            AuthDialogIntent::Logout => "Harness log out",
                        }}
                    </Title3>
                    <Body1>
                        "History e2e stub. Use lepton-uf-app for real credentials."
                    </Body1>
                    {match intent.get() {
                        AuthDialogIntent::Signin => view! {
                            <div data-testid="harness-auth-dialog-signin" />
                        }.into_any(),
                        AuthDialogIntent::Signup => view! {
                            <div data-testid="harness-auth-dialog-signup" />
                        }.into_any(),
                        AuthDialogIntent::Logout => view! {
                            <div data-testid="harness-auth-dialog-logout" />
                        }.into_any(),
                    }}
                    <Button appearance=ButtonAppearance::Subtle on_click=close>
                        "Close"
                    </Button>
                </div>
            }
            .into_any()
        }}
    }
}
