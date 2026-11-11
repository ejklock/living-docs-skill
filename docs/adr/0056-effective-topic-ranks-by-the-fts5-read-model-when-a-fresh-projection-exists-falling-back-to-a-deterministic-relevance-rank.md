---
type: ADR
title: effective --topic ranks by the FTS5 read-model when a fresh projection exists, falling back to a deterministic relevance rank
description: effective --topic layers onto the search FTS5 read-model for relevance ranking when a fresh projection is available (front resolves ranked paths, core filters+orders by them, liveness still excludes stale hits); with no or stale projection it falls back to a deterministic term-frequency rank — never erroring the read-only verb
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T13:57:31Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0056. effective --topic ranks by the FTS5 read-model when a fresh projection exists

## Context

ADR 0050 shipped `effective` with a deterministic substring filter for `--topic` and deferred FTS5 relevance ranking as a follow-up, once the deterministic core was proven. The core is proven and in force; this record takes that follow-up. The constraint is that `effective`'s value — chain collapse, stale exclusion, tiers, budget — must not regress, and the verb must never error just because no search projection exists (it is read-only and runs in file-mode by default).

## Decision

We will let `effective --topic` use the FTS5 read-model for relevance ordering when a fresh projection is available, and fall back to a deterministic rank otherwise:

- **Front resolves the ranking.** When `--topic` is given, the CLI front queries the search read-model (as `search` does), maps each hit to its `docs_dir`-prefixed path, and passes the ranked path list to core as `Options.ranked_topic`. Core restricts the view to those records and orders them by that rank.
- **Freshness gates it.** The front uses the projection only when its `sync_meta` fingerprint matches the records tree (mirroring `search`'s staleness check). A missing, unreachable, or stale projection prints one stderr warning and passes `None` — never an error.
- **Deterministic fallback.** With `ranked_topic: None` and a topic set, core ranks by a weighted term frequency (title ×3, description ×2, body ×1), keeping only matches. With no topic, the base constitution/PRD/contract rank stands.
- **Liveness still filters.** A stale or superseded record that FTS5 returns is still excluded from the default view — the ranked set restricts *order and membership within the active set*, it does not resurrect a record liveness withheld.

## Consequences

**Easier / gained:**
- `--topic` gets real relevance ordering where a projection exists, and a sensible deterministic ordering where it does not — the verb works in both modes without configuration.

**Harder / accepted trade-offs:**
- The front now does best-effort DB I/O for `--topic`; it double-connects (freshness, then search) like `search`, accepted for staying self-contained and never-erroring.

**Follow-ups:**
- None; this closes ADR 0050's FTS5 follow-up.

## Verification

**Implementation impact:** `living-docs-core/src/commands/effective.rs` (ordering by ranked set / deterministic relevance), `cli/src/commands/effective.rs` (FTS5 resolution + freshness fallback), `cli/src/args/sub.rs`.

**Verification criteria:**
- Given a ranked path set, `effective` returns only those records in that order and still excludes a stale one; with a topic and no ranked set it orders by the deterministic relevance score (a title match outranks a body-only match).
- With no or a stale projection, `effective --topic` exits 0 and prints the deterministic view; `check docs` over this repo stays green.
