---
type: Issue
title: Per-type doc size targets — authoring convention in the skill corpus + advisory warning in check, plus the same-change economic rationale
description: State per-doc-type body size targets (ADR/BDR/issue/research) as an authoring convention in the embedded living-docs skill corpus, have `check` emit an advisory (non-failing) warning when a body exceeds its target, and record the economic rationale for the docs-in-the-same-change rule where that rule lives.
status: open
labels: [slice, cli, skill-corpus, token-economy]
blocked_by: []
tracker:
timestamp: 2026-07-19T00:00:00Z
---

## Per-type doc size targets + the same-change economic rationale

Motivated by ai-configs research `0053`: output tokens scale with prose length, so
"write less" is the largest authoring saving available — and terse docs independently
read better and survive context compaction. The targets are a convention with an
advisory instrument, not a hard gate: judgment prose must never be truncated to satisfy
a number.

### Objective link

Constitution → [ADR 0001](/adr/0001-living-docs-cli.md) (`check` is the doc-gate) →
[ADR 0014](/adr/0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md)
(skill content lives in the embedded corpus) → this slice.

### Context manifest

- Read: `skills/living-docs/` corpus (authoring conventions topics), `check`
  implementation in `living-docs-core`.
- Seams touched: one new/extended corpus topic stating the per-type targets and their
  rationale; a `check` warning path (stdout note, exit 0) when a non-index doc body
  exceeds its type's target.
- Pattern: warning, never failure — mirrors how ai-configs states agent-file line
  targets as review pressure, not a build break.

### Scope

- Corpus: per-type body-line targets (proposed starting values, tune at implementation:
  ADR ≤ ~60, BDR ≤ ~40, issue ≤ ~80, research unbounded-but-index-summarized) with the
  one-line rationale (output-token cost + compaction survival) and the standing rule
  that a target is never a reason to omit a load-bearing rationale.
- Corpus: next to the docs-in-the-same-change rule, add the `0053` Finding-3 economic
  rationale — same-session hot-context writing is output-token-dominant; deferring docs
  re-pays the input context cold.
- `check`: advisory `SIZE` warning per over-target doc, listing doc, lines, target;
  exit code unchanged.

### Vertical Demo

- **Given** a bundle with an ADR body over target, **When** I run `living-docs check`,
  **Then** a `SIZE` advisory names the file and both numbers and the command still
  exits 0 (and existing failures still fail).
- **Given** `living-docs skill living-docs --topic <conventions-topic>`, **Then** the
  targets and the same-change economic rationale print from the embedded corpus.

### Acceptance

- `check` emits the advisory for an over-target fixture and stays exit 0; a fixture
  with a real invariant violation still exits non-zero. — verify_by: test
- Targets and rationale live in exactly one corpus topic (one home per fact) and are
  served via `living-docs skill`. — verify_by: command
- Complexity + no-comments + tests-with-the-change standing rules hold. — verify_by: command

### Out of scope

No hard failure on size, no auto-truncation, no per-project target overrides until a
real need appears. The `brief` scaffold is slice
[0008](/issues/0008-brief-scaffold.md).
