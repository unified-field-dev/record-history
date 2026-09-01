# history-ui-e2e

Leptos + Playwright host for `HistoryTimeline`. Timeline pages compile into the
main WASM so `cargo leptos end-to-end` does not need `--split`.

Auth is a tower-sessions harness (`seedAuth`), not lepton credentials. Users
and parent records upsert at boot and on `POST /api/test/seed-data`. History
rows are created once (the fixture table blocks update).

Hydrate and browser coverage for the timeline lives here. The oneshot host
`timeline-host` stays Axum-only (no WASM).

## Scenario catalog

| ID | Kind | Asserts |
|----|------|---------|
| `pw-history-unauth-gated-sad` | sad | Anon owned-source timeline shows access denied |
| `pw-history-owner-timeline-happy` | happy | Owner sees seeded owned-source change line |
| `pw-history-peer-guessed-id-sad` | sad | Outsider with the owner's RecordId is denied |
| `pw-history-auth-public-happy` | happy | Outsider can read AUTHENTICATED parent history |
| `pw-history-empty-happy` | happy | Authorized empty parent shows empty overlay, not deny |
| `pw-history-load-failed-sad` | sad | Harness load fault surfaces load-failed, not ACL deny |
| `pw-history-page-scroll-happy` | happy | Infinite scroll loads a second page of seeded rows |
| `pw-history-kind-filter-happy` | happy | `kind_filter` shows fixture kind only |
| `pw-history-kind-absent-empty-happy` | happy | Absent kind filter is empty, not deny |
| `pw-history-renderers-happy` | happy | Custom `kind_views` row + fallthrough for alt kind |

## Seed body

`POST /api/test/seed-data` JSON:

- `auth`: `anonymous` \| `owner` \| `outsider`
- `scenario`: `default` \| `empty` \| `multipage` \| `multikind` \| `load_fail`
- `fault`: optional bool (also set by `load_fail`)

Routes beyond `/history/:table/:id`:

- `/history/renderers/:table/:id` — fixture `kind_views`
- `/history/kind/:table/:id` — filter to `e2e_record_history_fixture`
- `/history/kind-absent/:table/:id` — filter to a missing kind

## Run

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
cd examples/history-ui-e2e/end2end && npm ci && npx playwright install chromium
cd ../../..
cargo leptos end-to-end --project history-ui-e2e
```

`cargo leptos end-to-end` is the gate. Hydrate uses the workspace
`wasm-release` profile. The SSR host uses `release` so rust-lld does not
bus-error on the debug graph. Workspace `.cargo/config.toml` sets
`getrandom_backend="wasm_js"` for the WASM target. It builds SSR + hydrate,
starts the host, and runs Playwright. Let it finish; do not interrupt it.

## Look next

[`examples/timeline-host`](../timeline-host/) for the Axum oneshot query + format
contract without WASM.
