# record-history-leptos

Paginated Leptos `HistoryTimeline` on top of the `record-history` domain crate.

Embed on detail pages; pages load via SSR `get_record_history_page` and render with
Orbital timeline chrome. Optional `HistoryRenderers` register per-kind overrides.

## Documentation

- Crate rustdoc: `cargo doc -p record-history-leptos --features ssr --open`
  (Features, Embed, Paginated SSR fetch, Register history renderers)
- Domain contract: `cargo doc -p record-history --features ssr --open`
- Root [`README.md`](../README.md) and [`docs/VERIFICATION.md`](../docs/VERIFICATION.md)
