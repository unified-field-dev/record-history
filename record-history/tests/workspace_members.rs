//! Gate: record-history / record-history-leptos / timeline-host are workspace members.
//!
//! Featureless sibling-source contract (lepton-shell / gauge / tag pattern).

use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn record_history_workspace_members_happy_path() {
    let root =
        fs::read_to_string(workspace_root().join("Cargo.toml")).expect("workspace Cargo.toml");
    for member in [
        "record-history",
        "record-history-leptos",
        "examples/timeline-host",
        "examples/history-ui-e2e",
    ] {
        assert!(
            root.contains(&format!("\"{member}\"")),
            "workspace must list {member}"
        );
        assert!(
            workspace_root().join(member).join("Cargo.toml").is_file(),
            "missing crate dir {member}"
        );
    }
}
