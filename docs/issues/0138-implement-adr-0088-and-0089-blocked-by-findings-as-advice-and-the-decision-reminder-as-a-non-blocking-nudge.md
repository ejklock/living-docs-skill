---
type: Issue
title: "Implement ADR 0088 and 0089: blocked_by findings as advice and the decision reminder as a non-blocking nudge"
description: Move every blocked_by finding to the advisory channel and turn the Stop hook into a PostToolUse additionalContext nudge with its install wiring
status: open
timestamp: 2026-11-15T23:09:45Z
---

## 0138. Implement ADR 0088 and 0089: blocked_by findings as advice and the decision reminder as a non-blocking nudge

### Scope

Implements [ADR 0088](/adr/0088-every-blocked-by-finding-is-advice-and-none-fails-check.md) and [ADR 0089](/adr/0089-the-decision-reminder-is-a-non-blocking-posttooluse-nudge-once-per-commit-cycle.md): the owner asked that hooks and dependency checks nudge rather than block.

Plan:
- S1: every `blocked_by` finding (dangling number, self-reference, cycle, non-numeric item) is a `blocked-by` advisory and never fails `check`; rules updated | files: living-docs-core/src/check/dependencies.rs, living-docs-core/src/check/dependencies/tests.rs, cli/tests/check_dependencies.rs, cli/tests/set_tracker_keys.rs, skills/living-docs/rules/issue-workflow.md, skills/living-docs/rules/check.md | after: none
- S2: decision-prompt.sh reads the PostToolUse payload (tool_name Edit or Write, the edited file path) and prints a PostToolUse additionalContext JSON at most once per HEAD, never a decision; opt-in and bundle rules unchanged | files: skills/living-docs/hooks/decision-prompt.sh, cli/tests/decision_prompt.rs | after: none
- S3: manage install hooks wires a PostToolUse entry with matcher Edit|Write instead of Stop and removes an earlier Stop entry; uninstall removes both; help, sub.rs docs, CONTRIBUTING.md and procedure.md describe the nudge | files: cli/src/hooks.rs, cli/src/hooks/settings.rs, cli/tests/hooks_install_stop.rs, cli/src/args/help.rs, cli/src/args/sub.rs, CONTRIBUTING.md, skills/living-docs/rules/procedure.md | after: none
- S4 (session): the constitution's gate line names a non-blocking nudge | files: docs/constitution.md | after: S2, S3
- Combined proof: rerun the 28-bundle comparison (no added violation) and replay a scripted session (one nudge per commit cycle, nothing blocked).

### Decision

- The three code slices touch disjoint files and run in parallel.

### Acceptance

- Each Verification criterion of ADR 0088 and ADR 0089 holds.
- `cargo test --workspace`, `make check`, `cargo fmt --all --check` and `se-gates check` pass.
