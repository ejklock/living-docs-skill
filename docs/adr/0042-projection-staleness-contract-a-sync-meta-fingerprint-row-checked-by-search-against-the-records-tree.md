---
type: ADR
title: "Projection staleness contract: a sync_meta fingerprint row, checked by search against the records tree"
description: the projection stores a sync_meta fingerprint row written on successful sync; search compares it against the records tree and warns or refuses when stale
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-08-27T19:07:17Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0042. Projection staleness contract: a sync_meta fingerprint row, checked by search against the records tree

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

<!-- The forces at play. What problem forced a decision? What constraints bound it?
     Written so a newcomer understands the pressure without prior knowledge. Link any
     research artifact or PRD/issue that motivates it, bundle-relative:
     [research](/research/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md). -->

The db-store projection is rebuilt only by an explicit `db sync` (ADR 0003, ADR 0004). Nothing tells `search` or the web front that the projection is behind the `.md` records. An agent that consumes a stale search result acts on it with confidence. [Issue 0034](/issues/0034-projection-freshness-sla-search-and-web-warn-or-refuse-on-a-stale-db-store-projection.md) demands a staleness signal keyed to the last SUCCESSFUL sync, so a silently stalled sync reads as stale even when no record changed.

Constraints: the check must be deterministic and cheap (it runs on every `search`); it must work for both engines (SQLite/FTS5 and ParadeDB); it applies only when the fs tree is the authoritative backend — in db-mode authoring the projection IS canonical and staleness does not apply.

## Decision

We will store one sync-metadata row in the projection and compare fingerprints at read time.

1. **`sync_meta` row.** A single-row table in the projection, written as the LAST step of a successful `db sync`: `last_sync_completed_at` (UTC), `tree_fingerprint` (hex SHA-256). A failed or interrupted sync never writes it, so the row always describes a completed sync.
2. **Fingerprint definition.** SHA-256 over the sorted list of `(relative_path, content_sha256)` pairs of every record file that feeds the projection, computed by one shared function in `living-docs-core`. Deterministic: same tree, same fingerprint.
3. **Read-time check.** `search` (fs-authoritative mode) recomputes the tree fingerprint and compares it to `sync_meta`. Match: silent. Mismatch or missing row: a one-line staleness warning on stderr naming `db sync` as the fix; results still print. `--strict` turns the mismatch into a nonzero exit with no results.
4. **Web front.** The web front reads the same `sync_meta` state and marks responses as stale; it never recomputes the fingerprint per request eagerly beyond a cheap cached check.

## Consequences

**Easier / gained:**
- A stalled or forgotten sync is visible at the moment of consumption — the agent-facing failure mode becomes "the index is stale" instead of a confidently wrong answer.
- The heartbeat is the completed sync, not the content change: steady data is not flagged, a dead pipeline cannot masquerade as fresh.

**Harder / accepted trade-offs:**
- Every `search` pays a tree hash of the records directory. Acceptable: corpora are hundreds of files, not millions.
- Renames and content-identical moves change the fingerprint and read as stale. Accepted: a `sync` after any tree change is the contract.

**Follow-ups:**
- Issue 0037 consumes this state as a Trusted-attribute signal in the doc-readiness scorecard.

## Verification

<!-- OPTIONAL — include when this decision must be honored in code, so the doc closes the
     doc → implement → verify loop an agent (and any review step) can consume. Omit
     for a purely advisory record. Keep criteria checkable, not aspirational.
     Implementation impact: files / modules this decision touches, e.g. `src/store.py`.
     Fitness function: the test / lint / arch-unit assertion that fails if the second
     verification criterion is violated (see `rules/adr-conventions.md` rule 6). -->

**Implementation impact:** `living-docs-core` (fingerprint function), `db-store` (sync writes `sync_meta`; both engines), `cli` (`search` check + `--strict`), `web` (staleness flag on responses).

**Verification criteria:**
- After `db sync`, `search --strict` on an unchanged tree exits zero with no warning.
- After any record edit without a sync, `search` warns on stderr and `search --strict` exits nonzero; a projection with no `sync_meta` row behaves as stale.
- Fitness function: an integration test runs sync → strict search (passes) → edits a record → strict search (fails) → sync → strict search (passes), per engine.

# References

[1] [Making Your Data Ready for Agentic AI — Sadalage & Chandrasekaran](https://martinfowler.com/articles/making-data-ready-for-agentic-ai.html)
