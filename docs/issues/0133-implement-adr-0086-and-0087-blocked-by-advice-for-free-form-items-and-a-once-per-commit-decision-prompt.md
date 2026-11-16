---
type: Issue
title: "Implement ADR 0086 and 0087: blocked_by advice for free-form items and a once-per-commit decision prompt"
description: "Land the two corrections from the QA friction pass: non-numeric blocked_by items become advisories, and the decision prompt fires at most once per commit cycle"
status: closed
timestamp: 2026-11-15T22:16:18Z
---

## 0133. Implement ADR 0086 and 0087: blocked_by advice for free-form items and a once-per-commit decision prompt

### Scope

Implements [ADR 0086](/adr/0086-blocked-by-fails-check-only-on-a-dangling-issue-number-a-self-reference-or-a-cycle-other-items-are-advice.md) and [ADR 0087](/adr/0087-the-decision-prompt-fires-at-most-once-per-commit-cycle.md), the two corrections from the 2026-11-15 QA friction pass.

Plan:
- S1: `check` reports a non-numeric `blocked_by` item as an advisory instead of a violation and leaves it out of the dependency graph; a scalar comma list is split into items; `write set <issue> blocked_by` accepts any item again, still writing numeric items as numbers; the issue-workflow rule says so | files: living-docs-core/src/check/dependencies.rs, living-docs-core/src/check/dependencies/tests.rs, living-docs-core/src/commands/set/issue_keys.rs, skills/living-docs/rules/issue-workflow.md, cli/tests/check_dependencies.rs, cli/tests/set_blocked_by_numbers.rs | after: none
- S2: the decision prompt keys its state on HEAD, prompting at most once per commit cycle | files: skills/living-docs/hooks/decision-prompt.sh, cli/tests/decision_prompt.rs | after: none
- Combined proof, after both: rerun the 28-bundle comparison against 0.28.0 and the scripted session replay.

### Decision

- The two slices touch disjoint files and run in parallel.

### Acceptance

- Each Verification criterion of ADR 0086 and ADR 0087 holds.
- The 28-bundle comparison adds no violation relative to 0.28.0 except numeric dangling items, self-references and cycles.
- `cargo test --workspace`, `make check`, `cargo fmt --all --check` and `se-gates check` pass.
