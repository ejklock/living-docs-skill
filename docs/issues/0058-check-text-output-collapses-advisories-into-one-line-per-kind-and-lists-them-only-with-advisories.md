---
type: Issue
title: check text output collapses advisories into one line per kind and lists them only with --advisories
description: The plain-text check report prints violations first, then one summary line per advisory kind with a count and the first record numbers; the full advisory list returns with --advisories, and the JSON shape is unchanged.
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0058. check text output collapses advisories into one line per kind and lists them only with --advisories

A `check` over a mature bundle prints one line per advisory before the verdict: the relent bundle prints 31 `MOVED-SOURCE` lines of about 250 characters each, every time, around a one-line verdict. Agents answer with `| tail -3` and `| head -c 300`, and the owner's transcripts show 1,027 `check` results that neither passed nor failed visibly because the output was cut. The gate's signal, the violation, must come first and the review queue of [ADR 0044](/adr/0044-moved-source-review-queue-check-emits-a-warn-level-finding-when-a-linked-record-is-superseded-or-demoted.md) must stay visible without drowning it.

### Scope

- Text output prints violations first, then one summary line per advisory kind: the count, the kind, the first five record numbers and `(+N)`, and the flag that lists them.
- A new `check --advisories` flag prints the full advisory list, one line per finding, after the summary.
- Violation and advisory lines print paths relative to the bundle root named on the first line.
- The JSON output, the verdict line, the exit codes and `--changed-files` scoping are unchanged.

### Decision

- Summaries sort by count, descending; option not taken: alphabetical by kind.
- The full list stays reachable through a flag rather than through `--json` alone, so a human at a terminal has the same path as an agent; option not taken: dropping the text list and pointing at `--json`.

### Acceptance

- A fixture bundle with one violation and three advisories of two kinds prints the violation line, then two summary lines, then the verdict; the summary of the kind with two findings names both record numbers.
- The same bundle with `--advisories` also prints the three advisory lines.
- A bundle with no advisories prints no summary line.
- `--json` output is byte-identical to the previous release's for the same bundle (a fixture test).
- The exit code is 1 with a violation and 0 without, with or without `--advisories`.
- The full suite and `make check` pass.

### Outcome

Delivered, uncommitted, in the working tree: `check` text output prints violations first, then one summary line per advisory kind (`2 SIZE advisories: 0017, 0029`, first five records then `(+N)`), one hint line, and the full advisory list only under `--advisories`; finding lines show bundle-relative paths; JSON output is byte-identical with and without the flag. Rendering lives in `cli/src/commands/check/render.rs`. Proof: `cargo test -p living-docs --test check_advisory_summary` (8 passed), `cargo test --workspace` green, clippy and fmt clean. The `check` guide topic (`skills/living-docs/rules/check.md`) describes the summary form.
