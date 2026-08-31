# timeline-host

Audited model host: seed fixture mutations, query via `history_for_source`,
format timeline lines under protected **`/history`**.

Production Leptos hosts embed `<HistoryTimeline />` on detail pages; the SSR
fetch (`GetRecordHistoryPage`) requires an authenticated session. This example
proves fixture writes + `history_for_source` + `format_line` without the
SSR/WASM / Orbital graph. The oneshot path `/history` is a teaching stand-in
for a protected host API; the embed surface stays `HistoryTimeline` +
`record-history-timeline` testid.

| | |
|---|---|
| **When to use** | First smoke of record-history query + format helpers in a host |
| **Command** | `CARGO_BUILD_JOBS=1 CARGO_TARGET_DIR=target-record-history cargo run -p timeline-host` |
| **Success** | Stdout: `timeline_host: OK — /history deny/allow + seeded timeline` |
| **Look next** | Embed [`HistoryTimeline`](../../record-history-leptos/) on product detail pages |

**Open first:** [`src/main.rs`](src/main.rs)

## Copy into your host

| File | What to take |
|------|----------------|
| This [`Cargo.toml`](Cargo.toml) | Axum oneshot shape + `record-history` / `lepton` `ssr` (query / format smoke) |
| Product mount `Cargo.toml` (below) | `record-history` + `record-history-leptos` with `ssr` / `hydrate` features |
| [`src/main.rs`](src/main.rs) | Fixture upsert, `history_for_source`, `format_line`, protect a host API |
| Leptos sketch (below) | `<HistoryTimeline />` on a detail page |

### Product mount dependencies

```toml
[dependencies]
record-history = { git = "https://github.com/unified-field-dev/record-history", package = "record-history", rev = "REPLACE_WITH_PIN", default-features = false }
record-history-leptos = { git = "https://github.com/unified-field-dev/record-history", package = "record-history-leptos", rev = "REPLACE_WITH_PIN", default-features = false }
uf-product = { /* your pin */, default-features = false }
uf-integrations = { /* your pin */, default-features = false }

[features]
ssr = [
    "record-history/ssr",
    "record-history-leptos/ssr",
    "uf-product/ssr",
    "uf-integrations/ssr",
]
hydrate = [
    "record-history-leptos/hydrate",
    "uf-product/hydrate",
    "uf-integrations/hydrate",
]
```

### Leptos mount sketch

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

Backend read (Leptos-free):

```rust,ignore
use record_history::{format_line, history_for_source};
use valence::RecordId;

let source = RecordId::new("tag", tag_id);
let rows = history_for_source(&source, &valence).await?;
let line = format_line(&rows[0], "Alice Chen", Some("/user/alice"));
```

Product tables opt in with `traits: [RecordHistory]`; parents use
`traits: [HistorySource]`. Writes usually go through Valence side effects (for
example Tag's `TagHistoryWriter`). Inventory names match the embed contract:
`HistoryTimeline`, `record-history-timeline`, `require_session` on
`GetRecordHistoryPage`.

For shell chrome (layout, fonts, Axum + Leptos boot), copy
[`shell-chrome-host`](https://github.com/unified-field-dev/unified-field-product/tree/main/examples/shell-chrome-host)
from unified-field-product, then embed `HistoryTimeline` on detail pages.

## Run (documented gate)

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
cargo check -p timeline-host
cargo run -p timeline-host
```

**Success:** stdout prints `timeline_host: OK — /history deny/allow + seeded timeline`.

## Hydrate / browser

Out of gate for this host. Full timeline UI needs a product binary with
`cargo-leptos`, `wasm32`, session chrome, and a working Orbital / `uf-product`
graph. Prefer the oneshot above.
