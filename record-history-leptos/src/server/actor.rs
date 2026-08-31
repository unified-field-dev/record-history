#[cfg(feature = "ssr")]
use anyhow::Context;
#[cfg(feature = "ssr")]
use lepton::generated::User;
#[cfg(feature = "ssr")]
use valence::{extract_id_from_record, Model, Valence};

/// Resolve actor display label and optional profile link.
///
/// Uses the **request** Valence only (never System). Prefer profile display name
/// when the viewer may read it; otherwise return an opaque label — never email.
/// Profile links are only emitted for safe bare user ids (`/user/{id}`).
#[cfg(feature = "ssr")]
pub async fn resolve_actor_presentation(
    actor: Option<valence::RecordId>,
    valence: &Valence,
) -> anyhow::Result<(String, Option<String>)> {
    let Some(rid) = actor else {
        return Ok(("System".into(), None));
    };

    let user_id = extract_id_from_record(&rid).context("actor record id")?;
    let bare_user_id = valence::ownership::normalize_record_id_for_ownership(&user_id);
    let href = actor_profile_href(&bare_user_id);

    match User::get(&bare_user_id, valence).await {
        Ok(Some(user)) => {
            let name = user
                .get_profile(valence)
                .await
                .unwrap_or_default()
                .into_iter()
                .next()
                .map(|p| p.display_name().to_string())
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| opaque_actor_label(&bare_user_id));
            Ok((name, href))
        }
        _ => Ok((opaque_actor_label(&bare_user_id), href)),
    }
}

/// Build `/user/{id}` only when `bare_user_id` is a single safe path segment.
///
/// Rejects empty ids and ids containing `/`, `\`, `?`, `#`, whitespace, or a
/// `://` smuggle so a poisoned actor RecordId cannot open-redirect the timeline.
#[cfg(feature = "ssr")]
#[must_use]
pub fn actor_profile_href(bare_user_id: &str) -> Option<String> {
    if !is_safe_user_path_segment(bare_user_id) {
        return None;
    }
    Some(format!("/user/{bare_user_id}"))
}

#[cfg(feature = "ssr")]
fn is_safe_user_path_segment(id: &str) -> bool {
    if id.is_empty() || id.contains("://") {
        return false;
    }
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

#[cfg(feature = "ssr")]
fn opaque_actor_label(bare_user_id: &str) -> String {
    let short: String = bare_user_id.chars().take(8).collect();
    if short.is_empty() {
        "User".into()
    } else {
        format!("User {short}")
    }
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{actor_profile_href, is_safe_user_path_segment};

    #[test]
    fn actor_profile_href_allows_safe_ids_happy_path() {
        assert_eq!(
            actor_profile_href("alice-01"),
            Some("/user/alice-01".into())
        );
        assert_eq!(
            actor_profile_href("user_42.x"),
            Some("/user/user_42.x".into())
        );
    }

    #[test]
    fn actor_profile_href_rejects_path_and_url_smuggle_sad() {
        assert!(!is_safe_user_path_segment(""));
        assert!(!is_safe_user_path_segment("../admin"));
        assert!(!is_safe_user_path_segment("a/b"));
        assert!(!is_safe_user_path_segment("a\\b"));
        assert!(!is_safe_user_path_segment("a?x=1"));
        assert!(!is_safe_user_path_segment("a#frag"));
        assert!(!is_safe_user_path_segment("https://evil.example"));
        assert!(!is_safe_user_path_segment("a b"));
        assert_eq!(actor_profile_href("../admin"), None);
        assert_eq!(actor_profile_href("https://evil.example"), None);
    }
}
