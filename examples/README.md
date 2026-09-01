# Examples

Runnable teaching hosts for this product. Each card: when to use · command ·
success · look next. Copy `Cargo.toml` + `main.rs` (and the product mount
snippets in the host README) into your composite host.

## Canonical path

### `timeline-host` — seeded mutations + timeline

**Teaches:** audited fixture writes, `history_for_source`, and `format_line`
under protected `/history`. Inventory names match the embed contract
(`HistoryTimeline`, `record-history-timeline`, `require_session` on
`GetRecordHistoryPage`).

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
cargo run -p timeline-host
```

**Success:** stdout prints `timeline_host: OK — /history deny/allow + seeded timeline`.

**Next step:** Embed `<HistoryTimeline />` from `record-history-leptos` on detail pages.
Copy table + product mount `Cargo.toml`: [`timeline-host/README.md`](timeline-host/README.md).

| Host | When to use | Command | Success | Look next |
|------|-------------|---------|---------|-----------|
| [`timeline-host`](timeline-host/) | Audited model + timeline | `cargo run -p timeline-host` | Deny/allow + 3 timeline lines | `HistoryTimeline` in product UI |
