---
type: ADR
title: "Effective view: a read verb compiling active records with supersede chains collapsed, progressive tiers, and a hard token budget"
description: effective compiles the agent-facing view of the bundle — active records only, supersede chains collapsed to the head with a lineage line, index/outline/full tiers ranked constitution/PRD/contract-first, capped by a hard --budget — so agents read the effective view, never the raw append-only corpus
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:08:36Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0050. Effective view: a read verb compiling active records with supersede chains collapsed, progressive tiers, and a hard token budget

## Context

The corpus is append-only by design (supersede, never rewrite) — right for history, wrong as the thing an agent reads. On a strict-mode consumer the bundle is ~506k words; no agent reads it, so agents read pointers, and pointers land on records a later record already superseded (chains like 0131 → 0133 → 0143 where the rule in force lives only in the last link). The skill already prepares the write side (`brief` fills only judgment slots; `index` is generated); the read side has no equivalent. `search` returns hits and `index.md` lists everything — the agent does the resolution by hand.

## Decision

We will add an `effective` verb that compiles the agent-facing view over the record set, derived and never committed (the records stay the SSOT, like the generated indexes):

- **Active only.** `Superseded`/`Deprecated` records, and records the liveness check (ADR 0049) flags stale, are excluded from the default output; `--include-stale` restores them.
- **Chains collapsed to the head.** A superseded record is dropped; the surviving head carries a one-line lineage (`supersedes 0131 via 0133`) computed by walking `supersedes` through the dropped ancestors, rather than three documents.
- **Progressive tiers.** `--tier index` (title + description per record, the default), `--tier outline` (headings), `--tier full` (bodies).
- **Hard token budget.** `--budget <tokens>` is a cap, not a suggestion (tokens estimated at ~4 chars each): truncation degrades the tier first (full → outline → index), then drops the lowest-ranked records, so output never overflows the caller's window.
- **Ranking.** Constitution and PRDs first, then contracts (a `## Verification` block) above narrative, then by number — so the agent orients before it drills in.
- **Query.** `effective --topic <term>` filters the set by a case-insensitive term match; `effective` with no topic returns everything active. FTS5-ranked retrieval stays `search`'s job — `effective` adds resolution, ranking and budgeting over the record set, deterministically and without a database.

## Consequences

**Easier / gained:**
- Agents read one compiled, in-force, budget-bounded view instead of the raw history, so a pointer can no longer land on a superseded rule.

**Harder / accepted trade-offs:**
- `--topic` is a deterministic term filter, not FTS5 relevance ranking. Accepted for this slice: the resolution/collapse/budget value does not need the index, and `search` already serves ranked retrieval; FTS5-backed topic ranking is a follow-up.

**Follow-ups:**
- Layer `effective --topic` onto the FTS5 read-model for relevance ranking once the deterministic core is proven.

## Verification

**Implementation impact:** `living-docs-core/src/commands/effective.rs` (new) and its `render` submodule, `cli/src/commands/effective.rs` (new), `cli/src/args.rs` (`effective` subcommand), `skills/living-docs/rules/semantic-index.md`, `skills/living-docs/SKILL.md`.

**Verification criteria:**
- On a three-link supersede chain, `effective --topic <term>` returns the head only, with the lineage line; `--budget N --tier full` never exceeds `N`, degrading tier then dropping lowest-ranked records.
- Stale records (ADR 0049) are absent from the default output and present under `--include-stale`.
