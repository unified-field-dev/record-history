# Record History

[![CI](https://github.com/unified-field-dev/record-history/actions/workflows/ci.yml/badge.svg)](https://github.com/unified-field-dev/record-history/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

[GitHub](https://github.com/unified-field-dev/record-history) · `cargo doc -p record-history --features ssr --open`

## About

Record History is the Unified Field **shared audit timeline**: Valence
`RecordHistory` / `HistorySource` traits, cross-table query helpers, and a
paginated Leptos `HistoryTimeline` for detail pages.

- **Domain (`record-history`)** — trait schemas, `RecordHistoryQueryAll`,
  `history_for_source`, formatting, row identity for renderer routing
- **Timeline UI (`record-history-leptos`)** — SSR `get_record_history_page` +
  embeddable `HistoryTimeline`
- **Composition** — product tables opt in with `traits: [RecordHistory]`;
  parents use `traits: [HistorySource]`; writes usually go through Valence
  side effects

Crate-root rustdoc owns the Features inventory and Get started guides. Start at
`cargo doc -p record-history --features ssr --open`, then
`record-history-leptos` with `--features ssr`.

## Getting started

```toml
[dependencies]
record-history = { git = "https://github.com/unified-field-dev/record-history", package = "record-history", branch = "main" }
record-history-leptos = { git = "https://github.com/unified-field-dev/record-history", package = "record-history-leptos", branch = "main", default-features = false }
```

```rust,ignore
use record_history_leptos::HistoryTimeline;
use valence::RecordId;

view! {
    <HistoryTimeline source=RecordId::new("tag", tag_id) />
    // optional narrow:
    <HistoryTimeline
        source=RecordId::new("tag", tag_id)
        kind_filter=vec!["tag_history".into()]
    />
}
```

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
cargo test -p record-history --features ssr
```

## Workspace

| Crate | Role |
|-------|------|
| [`record-history`](record-history/) | Trait schemas, queries, format helpers |
| [`record-history-leptos`](record-history-leptos/) | Paginated SSR + `HistoryTimeline` |

## Examples

| Host | When to use | Command | Success | Look next |
|------|-------------|---------|---------|-----------|
| [`timeline-host`](examples/timeline-host/) | Audited mutations + timeline | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-record-history cargo run -p timeline-host` | Deny/allow + seeded lines | Embed `HistoryTimeline` |

Copy `Cargo.toml` + `main.rs` (and the product-mount feature graph) from the
host README. More examples: [`examples/README.md`](examples/README.md).

## Security

History read privacy, session gates, and reporting: [`SECURITY.md`](SECURITY.md).
Report vulnerabilities privately — do not open a public issue for
security-sensitive reports.

## Verify

GitHub Actions (`.github/workflows/ci.yml`) runs the CI subset from
[`docs/VERIFICATION.md`](docs/VERIFICATION.md): fmt, clippy on `record-history`
(+ teaching host), contract tests, `timeline-host` check/run, and record-history
rustdoc with broken-intra-doc-link deny.

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
cargo fmt -p record-history -p timeline-host -- --check
cargo clippy -p record-history --all-targets --features ssr -- -D warnings
cargo clippy -p timeline-host --all-targets -- -D warnings
cargo test -p record-history --test workspace_members --test product_surface
cargo test -p record-history --features ssr --test history_api_contract --test record_history_integration --test source_deletion_integration --test source_model_nav_integration --test source_pagination_integration --test source_query_hops_integration --test source_refinement_integration --test sqlite_file_fixture_upsert
cargo check -p timeline-host
cargo run -p timeline-host
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p record-history --features ssr --no-deps
```

Teaching host success line:
`timeline_host: OK — /history deny/allow + seeded timeline`.
Contribute: [`CONTRIBUTING.md`](CONTRIBUTING.md).

## FAQ

**Is it a standalone server?** No. `record-history` is the domain library;
`record-history-leptos` mounts into a composite host that already wires Valence
and session chrome.

**Do I need the timeline UI?** No. Backend hosts can depend on `record-history`
alone and call `history_for_source` / `RecordHistoryQueryAll`. Embed
`HistoryTimeline` when operators need the scroll UI.

**How do products write history?** Concrete tables with `traits: [RecordHistory]`
use generated `Model::create` or a Valence side effect (for example Tag's
`TagHistoryWriter`). This crate does not invent product-specific write APIs.

**Where does privacy come from?** Prefer `history_for_source` /
`authorize_history_source_read` (HistorySource gate). Raw `RecordHistoryQueryAll`
may bypass that helper. History tables also use `read: { defer_to_edge: "source" }`
so Valence re-checks the parent record's Read policy on each satellite row —
see [`SECURITY.md`](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE), [CONTRIBUTING.md](CONTRIBUTING.md),
[SECURITY.md](SECURITY.md), and [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
