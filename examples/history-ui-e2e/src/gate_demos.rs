//! Harness auth bootstrap: tower-sessions e2e keys → [`AuthContext`] (no lepton Backend).

use leptos::prelude::*;
use uf_product::models::auth::{AnonymousUser, AuthSession, AuthenticatedUser};
use uf_product::{provide_auth_context, provide_auth_dialog_controller};

#[cfg(feature = "ssr")]
const E2E_AUTH_KEY: &str = "e2e_auth_kind";

/// Higgs / Valence record id for the owner e2e user (`table:id`).
#[cfg(feature = "ssr")]
pub const E2E_OWNER_SESSION_USER: &str = "user:e2e-user";
/// Higgs / Valence record id for the outsider e2e user (`table:id`).
#[cfg(feature = "ssr")]
pub const E2E_OUTSIDER_SESSION_USER: &str = "user:e2e-outsider";

/// E2e session kinds stored under [`E2E_AUTH_KEY`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum E2eAuthKind {
    /// Signed out / anonymous.
    Anonymous,
    /// Owner of the seeded owned-source parent.
    Owner,
    /// Authenticated user who does not own the seeded owned-source parent.
    Outsider,
}

impl E2eAuthKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anonymous => "anonymous",
            Self::Owner => "owner",
            Self::Outsider => "outsider",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw {
            "owner" | "authenticated_verified" => Self::Owner,
            "outsider" => Self::Outsider,
            _ => Self::Anonymous,
        }
    }

    /// Higgs `SessionSnapshot` user id (`table:id`), when signed in.
    #[cfg(feature = "ssr")]
    pub const fn session_user_id(self) -> Option<&'static str> {
        match self {
            Self::Anonymous => None,
            Self::Owner => Some(E2E_OWNER_SESSION_USER),
            Self::Outsider => Some(E2E_OUTSIDER_SESSION_USER),
        }
    }

    pub fn to_session(self) -> AuthSession {
        match self {
            Self::Anonymous => AuthSession::Anonymous(AnonymousUser { reason: None }),
            Self::Owner => AuthSession::Authenticated(AuthenticatedUser {
                user_id: "e2e-user".into(),
                email: Some("e2e@example.com".into()),
                display_name: Some("E2E Owner".into()),
                avatar_url: None,
                roles: vec!["user".into()],
                email_verified: true,
            }),
            Self::Outsider => AuthSession::Authenticated(AuthenticatedUser {
                user_id: "e2e-outsider".into(),
                email: Some("outsider@example.com".into()),
                display_name: Some("E2E Outsider".into()),
                avatar_url: None,
                roles: vec!["user".into()],
                email_verified: true,
            }),
        }
    }
}

/// Fetch e2e auth kind from the server session.
#[server(E2eGetAuthKind)]
pub async fn e2e_get_auth_kind() -> Result<String, ServerFnError> {
    #[cfg(feature = "ssr")]
    {
        use leptos_axum::extract;
        use tower_sessions::Session;

        let session: Session = extract().await?;
        let kind = session
            .get::<String>(E2E_AUTH_KEY)
            .await
            .map_err(|e| ServerFnError::new(e.to_string()))?
            .unwrap_or_else(|| E2eAuthKind::Anonymous.as_str().to_string());
        Ok(kind)
    }
    #[cfg(not(feature = "ssr"))]
    {
        Ok(E2eAuthKind::Anonymous.as_str().to_string())
    }
}

/// Resolve e2e session → [`AuthContext`] under Suspense so SSR and hydrate agree.
#[component]
pub fn E2eAuthProvider(children: ChildrenFn) -> impl IntoView {
    let auth_kind = Resource::new(
        || (),
        |_| async move {
            e2e_get_auth_kind()
                .await
                .unwrap_or_else(|_| E2eAuthKind::Anonymous.as_str().to_string())
        },
    );

    view! {
        <Suspense fallback=|| {
            view! { <div data-testid="e2e-auth-loading" style="display:none" /> }
        }>
            {move || {
                let kind = auth_kind
                    .get()
                    .map(|raw| E2eAuthKind::parse(&raw))
                    .unwrap_or(E2eAuthKind::Anonymous);
                let _auth = provide_auth_context(kind.to_session());
                let _auth_dialog = provide_auth_dialog_controller();
                view! {
                    <div
                        data-testid="e2e-auth-bootstrap"
                        data-auth=kind.as_str()
                        style="display:none"
                    />
                    {children()}
                }
            }}
        </Suspense>
    }
}

#[cfg(feature = "ssr")]
pub async fn write_e2e_auth_kind(
    session: &tower_sessions::Session,
    kind: E2eAuthKind,
) -> anyhow::Result<()> {
    session
        .insert(E2E_AUTH_KEY, kind.as_str().to_string())
        .await
        .map_err(|e| anyhow::anyhow!(e.to_string()))
}

/// Mirror the e2e tower-session into `higgs_identity::SessionSnapshot`.
#[cfg(feature = "ssr")]
pub async fn inject_e2e_session_snapshot(
    session: tower_sessions::Session,
    mut req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use higgs_identity::SessionSnapshot;

    let kind = session
        .get::<String>(E2E_AUTH_KEY)
        .await
        .ok()
        .flatten()
        .map(|raw| E2eAuthKind::parse(&raw))
        .unwrap_or(E2eAuthKind::Anonymous);
    if let Some(user_id) = kind.session_user_id() {
        req.extensions_mut()
            .insert(SessionSnapshot::new(user_id, b"e2e-auth-hash"));
    }
    next.run(req).await
}
