---
type: Issue
title: "Implement ADR 0083: the opt-in decision prompt Stop hook and its installation"
description: "Land ADR 0083: the decision-prompt script, its install and uninstall wiring in a new module, the constitution line and the 20-prompt measurement"
status: open
timestamp: 2026-11-15T17:33:57Z
---

## 0126. Implement ADR 0083: the opt-in decision prompt Stop hook and its installation

### Scope

Implements [ADR 0083](/adr/0083-an-opt-in-stop-hook-prompts-the-agent-to-record-a-material-choice-in-the-active-issue-before-the-turn-ends.md): an opt-in Stop hook that prompts the agent, once per distinct change set, to record a material choice in the active issue's Decision section. Slice S4 of [issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

Plan:
- S1: `skills/living-docs/hooks/decision-prompt.sh` prints nothing and exits 0 unless `LIVING_DOCS_DECISION_PROMPT=1`; with it set it blocks once (Claude Code Stop hook JSON) when non-bundle paths changed, no bundle path changed and the stop-hook-active flag is false, keyed by a hash of the changed paths and contents stored under the git directory; the message names the issue number when the branch carries one | files: skills/living-docs/hooks/decision-prompt.sh (create), cli/tests/decision_prompt.rs (create) | after: none
- S2: `manage install hooks` writes the script and wires a Stop entry in `.claude/settings.json`; `manage uninstall` removes both; the wiring lives in a new module so `cli/src/hooks.rs` (521 lines, over the 300-line ratchet) does not grow; `rules/procedure.md` documents the opt-in | files: cli/src/hooks.rs, cli/src/hooks/ (new module), cli/tests/hooks_install.rs, skills/living-docs/rules/procedure.md | after: S1
- S3 (session): amend the constitution's enforcement line to name the opt-in prompt | files: docs/constitution.md | after: S2 (done)
- S4 (owner): run it in one consumer project and count the first 20 prompts against the kill criterion | after: S3

### Decision

- The hook ships for Claude Code only; other harnesses wait for the experiment's result (ADR 0083).
- Uninstall restores settings as before install with an empty hook section treated as absent, as Claude Code reads it; a user's own empty section that install appended to is removed on uninstall (review of S2). Not taken: recording which sections install created.
- The wiring of S2 is born in its own module because the file-size ratchet forbids `cli/src/hooks.rs` from growing.

### Acceptance

- Each Verification criterion of ADR 0083 holds.
- `cargo test --workspace`, `make check`, `cargo fmt --all --check` and `se-gates check` pass.
- The S4 count is recorded in this issue before it closes.
