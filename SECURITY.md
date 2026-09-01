# Security Policy

## Supported versions

Security fixes are accepted against the latest published `0.1.x` release line of this repository's crates (`record-history`, `record-history-leptos`).

## Reporting a vulnerability

Please **do not** open a public GitHub issue for security-sensitive reports.

Prefer one of the following:

1. **GitHub Security Advisories** — use [Report a vulnerability](https://github.com/unified-field-dev/record-history/security/advisories/new) on this repository when available.
2. Contact the maintainers privately via the repository owner listed at https://github.com/unified-field-dev/record-history.

Include:

- a description of the issue and its impact
- steps to reproduce or a proof of concept when possible
- affected crate names and versions

We will acknowledge receipt as soon as practical and coordinate a fix and disclosure timeline with you.

## Scope

In scope: vulnerabilities in this repository's published crates and documentation that could cause unsafe production defaults, plus CI/supply-chain issues in this repository.

Out of scope: vulnerabilities solely in third-party dependencies unless this project mishandles them in a security-relevant way.

## History privacy

**SEC-HISTORY-PARENT:** timeline reads require authorization against the source/parent
record. Session presence alone is not enough. If the source cannot be authorized,
the read fails closed (`Not authorized to view this history`).

Happy path: the owner or another actor who can Read the parent can page its
history. Sad path: a peer who guesses a `RecordId` cannot page another record's
history. Missing parents and tables that are not `HistorySource` implementors
are denied the same way.

Visibility follows the **parent** table's Read policy. Parents whose Read allows
`AUTHENTICATED` stay readable to any signed-in actor. Owner-scoped parents (this
crate's `e2e_history_source_owned` fixture, or any product table with owner Read)
deny peers.

**How it is enforced:** `authorize_history_source_read` / `history_for_source`
and `get_record_history_page` load the parent as System, then evaluate the
request actor against the parent table's Read policy. History **row** tables in
this crate use `read: { defer_to_edge: "source" }` so Valence also re-checks
parent Read on each satellite row; they do not re-encode ownership on the audit
row itself.

**Parent-edge privacy:** Prefer `history_for_source` /
`authorize_history_source_read` for product reads — those helpers still gate on
`HistorySource` membership and parent Read. Raw `RecordHistoryQueryAll` may
still bypass the helper; do not treat union query as an ACL boundary.

Counts and actor presentation use the request Valence (no System bypass). The
page DTO exposes a preformatted `change_line` **and** raw `old_value` /
`new_value` so product `HistoryRenderers` can build FieldDiff or Avatar rows
without a second Valence round-trip; client errors are sanitized. Page `limit`
is clamped to `1..=50`, `offset` is capped, kind filters are sanitized, and
count / kind-filter scans are hard-capped so forged page args cannot force an
unbounded history materialization. Actor profile links are emitted only for safe
`/user/{id}` path segments.

**Fixture / product history tables:** create and delete stay `SYSTEM_ONLY`; update
is blocked.
