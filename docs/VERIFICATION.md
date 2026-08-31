# record-history verification

Re-run after code or doc changes. This workspace is the Record History product
(`record-history` Valence trait helpers + `record-history-leptos` timeline UI).
Layer 1 covers history APIs, privacy contracts, and sibling-source UI surface
gates. Layer 2 is Playwright against `examples/history-ui-e2e`.

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-record-history
export CARGO_INCREMENTAL=0
export CARGO_PROFILE_DEV_DEBUG=0
export RUSTFLAGS="-D warnings"
```

This workspace pins `rust-toolchain.toml` to `nightly` (Leptos `nightly` features + Orbital).

## PR CI parity (`.github/workflows/ci.yml`)

Run these before push when touching this repo. Commands match CI jobs exactly.

### fmt

```bash
cargo fmt --all -- --check
```

### clippy

```bash
cargo clippy -p record-history --all-targets --features ssr -- -D warnings
cargo clippy -p record-history-leptos --all-targets --features ssr -- -D warnings
cargo clippy -p timeline-host --all-targets -- -D warnings
```

### test

```bash
cargo test -p record-history --test workspace_members --test product_surface
cargo test -p record-history --features ssr \
  --test history_api_contract \
  --test record_history_integration \
  --test source_deletion_integration \
  --test source_model_nav_integration \
  --test source_pagination_integration \
  --test source_query_hops_integration \
  --test source_refinement_integration \
  --test sqlite_file_fixture_upsert \
  --test privacy_policy_integration \
  --test parent_acl_integration
cargo check -p timeline-host
cargo run -p timeline-host
```

Success line: `timeline_host: OK — /history deny/allow + seeded timeline`.

### docs

```bash
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p record-history --features ssr --no-deps
```

### e2e

```bash
cd examples/history-ui-e2e/end2end && npm ci && npx playwright install chromium
cd ../../..
cargo leptos end-to-end --project history-ui-e2e
```

Let the command finish. Do not interrupt it. Hydrate uses the workspace
`wasm-release` profile. The SSR host uses `bin-profile-dev = "release"`.

Scenario IDs: `pw-history-unauth-gated-sad`, `pw-history-owner-timeline-happy`,
`pw-history-peer-guessed-id-sad`, `pw-history-auth-public-happy`,
`pw-history-empty-happy`, `pw-history-load-failed-sad`,
`pw-history-page-scroll-happy`, `pw-history-kind-filter-happy`,
`pw-history-kind-absent-empty-happy`, `pw-history-renderers-happy`.

### leptos-lints

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview
export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"
cargo dylint --all -p record-history-leptos --no-deps -- --features hydrate
```

## Layer 1 — full package (local, optional)

```bash
cargo test -p record-history --features ssr
```

## Layer 3 — Cloud / performance

**Waived.** Local product workspace. No cloud resources or Criterion benches.
Correctness is in-process against embedded SQLite (`:memory:` for contracts;
optional on-disk SQLite fixture upsert).

## Notes

- Tests may `unwrap`/`expect`; production server fns map failures to
  `ServerFnError` (no ordinary-path unwrap).
- Sad-path assertions check message content, `None`, or empty — stronger than
  `is_err()` alone.
- Happy-path tests are named `*_happy_path` so audits detect them.
