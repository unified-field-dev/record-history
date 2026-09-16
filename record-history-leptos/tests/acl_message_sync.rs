#![cfg(feature = "ssr")]
//! Regression: `record-history-leptos`'s client-visible ACL-denial constant
//! must stay byte-identical to `record-history`'s domain-level constant.
//!
//! `record-history-leptos::components::history_timeline::is_history_acl_error`
//! classifies a `ServerFnError` message string as "access denied" vs. "generic
//! failure" purely by string match (see `record-history-leptos`'s crate docs
//! and `SECURITY.md`). The two literals live in separate crates — this crate
//! cannot depend on `record-history` under `hydrate`-only builds, since
//! `record-history` is an `ssr`-only optional dependency — so nothing at
//! compile time ties them together. This test is the closest achievable
//! contract: an exhaustive equality check run every `ssr`-featured CI build.

#[test]
fn acl_denied_message_matches_domain_constant_happy_path() {
    assert_eq!(
        record_history_leptos::constants::HISTORY_ACL_DENIED_MSG,
        record_history::HISTORY_ACCESS_DENIED,
        "record-history-leptos's client-side ACL constant has drifted from \
         record-history::HISTORY_ACCESS_DENIED — update both together"
    );
}
