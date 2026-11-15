---
type: Issue
title: "Implement ADR 0082: blocked_by validation in check, write-time refusal and renumber rewrite"
description: "Land ADR 0082 in three disjoint slices: the dependency check pass, the write-time refusal of non-number items, and the renumber rewrite of dependents"
status: closed
timestamp: 2026-11-15T17:33:57Z
---

## 0125. Implement ADR 0082: blocked_by validation in check, write-time refusal and renumber rewrite

### Scope

Implements [ADR 0082](/adr/0082-blocked-by-names-local-issue-numbers-and-check-fails-on-a-dangling-dependency-a-self-reference-or-a-cycle.md): `blocked_by` names local issue numbers, `write set` refuses a non-number item, `check` fails on a dangling dependency, a self-reference or a cycle, and `write set <issue> number next` rewrites every dependent's `blocked_by`. Slice S5 of [issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

Plan:
- S1: check reports a blocked_by item that names no issue, an issue that lists itself, and every cycle naming each member; a closed blocker resolves; `12` and `0012` are one issue; per-issue findings honour `check --changed-files` while resolution reads every issue | files: living-docs-core/src/check/dependencies.rs (scaffolded), living-docs-core/src/check/dependencies/tests.rs (create if over ~100 lines), cli/tests/check_dependencies.rs (create) | after: none
- S2: `write set <issue> blocked_by` refuses an item that is not a number, naming it, and leaves the record unchanged; the key's help text and `rules/issue-workflow.md` stop calling it tracker-specific | files: living-docs-core/src/commands/set/issue_keys.rs, cli/src/args/help.rs, skills/living-docs/rules/issue-workflow.md, cli/tests/set_tracker_keys.rs | after: none
- S3: `write set <issue> number next` rewrites the renumbered issue's number in every other issue's `blocked_by` in the same run | files: living-docs-core/src/commands/set/renumber.rs, cli/tests/set_number.rs | after: none
- Combined flow, after all three: renumber an issue that others wait on, then `check` passes; `living-docs check` over this bundle and over a copy of the relent bundle stays green.

### Decision

- The pass is a violation, not an advisory, with no grace window (ADR 0082, accepted as written).
- The relent check criterion is restated: the released 0.28.0 binary already reports 132 pre-existing violations on that bundle (none about `blocked_by`), so the proof is that the new pass adds none, not that the bundle passes. Not taken: fixing the relent bundle here (another repository).
- A test fixture that writes a dangling `blocked_by` (cli/tests/set_tracker_keys.rs) gains the issues it names rather than filtering the new finding out of its assertion.
- The pass is called from `check_numbering` in `living-docs-core/src/check/mod.rs`, beside the duplicate-number pass, because both judge references by record number; the helper keeps `run_all_checks` within the 30-line limit.

### Acceptance

- Each Verification criterion of ADR 0082 holds, proved by the slice tests and the combined flow.
- `cargo test --workspace`, `make check`, `cargo fmt --all --check` and `se-gates check` pass.
