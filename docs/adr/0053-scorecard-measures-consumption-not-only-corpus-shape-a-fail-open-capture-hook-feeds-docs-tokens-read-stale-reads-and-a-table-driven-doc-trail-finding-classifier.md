---
type: ADR
title: "Scorecard measures consumption, not only corpus shape: a fail-open capture hook feeds docs-tokens-read, stale-reads, and a table-driven doc-trail finding classifier"
description: scorecard grows a consumption block fed by a fail-open observe-docs-read hook (JSONL) — docs tokens read (sum/median/p90), stale reads, and doc-trail finding share via a table-driven regex classifier that reports unclassified rather than guessing; missing capture is not-measured, scorecard still never fails
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:43:30Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0053. Scorecard measures consumption, not only corpus shape

## Context

No number says whether living-docs helps or hurts the agents that use it. `scorecard` grades projection freshness and corpus shape; the questions that decide `strict` vs `lite`, materiality (ADR 0052), and the effective view (ADR 0050) are consumption questions — how many tokens of docs a task reads, how often it reads a stale record, how many review findings are about the doc trail rather than behavior. Without those, every enforcement change is faith against faith. One audit had to reconstruct this by grepping transcripts; the categories it found (citation, envelope-protocol) are exactly what this tool should emit on its own.

## Decision

We will add a `consumption` block to `scorecard`, fed by a lightweight capture and never gating:

- **Capture.** An `observe-docs-read.sh` PostToolUse hook appends one JSONL line per Read/Grep/Glob under the bundle: path, record id, status at read time, approximate token count, task id. Fail-open, never blocks, and **off by default** — a project opts in by wiring it, because measurement is not free.
- **Metrics.** Docs tokens read (sum, median, p90) and stale reads (a read whose captured status was `Superseded`/`Deprecated`) come from the JSONL. Doc-trail finding share comes from a **table-driven** classifier over an optional findings JSONL: phrasings live in a committed file, matching is case-insensitive substring, and a miss is reported as `unclassified` rather than guessed.
- **Window.** `scorecard --since 7d` summarizes only reads within the window, measured back from the newest captured read so the summary is deterministic without a wall clock.
- **Never fails.** Missing capture reads as "not measured"; `scorecard` still always exits zero.

## Consequences

**Easier / gained:**
- Materiality (ADR 0052) and the effective view (ADR 0050) get acceptance numbers: doc-trail-finding share and docs tokens per task must not rise; stale reads should fall toward zero.

**Harder / accepted trade-offs:**
- The classifier is only as good as its phrasing table, so it reports `unclassified` rather than force a category — a visible miss beats a silent miscount.

**Follow-ups:**
- Wire a findings capture (review verdicts) so the doc-trail-finding share is populated, not just "not measured".

## Verification

**Implementation impact:** `skills/living-docs/hooks/observe-docs-read.sh` (new), `living-docs-core/src/commands/scorecard/consumption.rs` (new) and its `classifier` submodule, `cli/src/commands/scorecard.rs`, `cli/src/args/sub.rs` (`--since`).

**Verification criteria:**
- With a capture log, `scorecard --since 7d` prints docs tokens (sum/median/p90), stale reads, and the finding share; with none it prints "not measured" and still exits 0.
- The classifier is unit-tested on the phrasing samples and reports `unclassified` for an unmatched finding.
