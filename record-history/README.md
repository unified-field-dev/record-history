# Record History

Valence trait + helpers for audit timeline rows.

Shared audit columns via `valence_trait_schema! { RecordHistory }`. Product
tables opt in with `traits: [RecordHistory]`. Parent records use
`traits: [HistorySource]`.

The Leptos timeline UI lives in sibling crate `record-history-leptos`
(`HistoryTimeline`).

## Documentation

- Crate rustdoc: `cargo doc -p record-history --features ssr --open`
  (Organized by task, Owns, Concern → API, Examples)
- Root [`README.md`](../README.md) and [`docs/VERIFICATION.md`](../docs/VERIFICATION.md)
