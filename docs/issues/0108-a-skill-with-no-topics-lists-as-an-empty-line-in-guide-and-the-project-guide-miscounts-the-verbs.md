---
type: Issue
title: A skill with no topics lists as an empty line in guide, and the project guide miscounts the verbs
description: Make guide --list and guide --help say when a skill has no topics and how to read it, and state ten verbs plus completions in CLAUDE.md and README.md.
status: closed
timestamp: 2026-11-09T11:53:41Z
---

## 0108. A skill with no topics lists as an empty line in guide, and the project guide miscounts the verbs

`living-docs guide --list --plain` prints `living-docs-drift:` with nothing after the colon, and `living-docs guide --help` reads `living-docs-drift:;`, because the drift skill added in [ADR 0075](/adr/0075-doc-to-code-drift-is-an-advisory-skill-whose-llm-judge-runs-outside-the-binary-through-a-configurable-command.md) ships a `SKILL.md` with no `rules/` or `templates/`. A reader cannot tell that `living-docs guide living-docs-drift` prints that skill's `SKILL.md`. Separately, `CLAUDE.md` and `README.md` say the CLI has ten verbs while `--help` lists eleven: [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) fixed ten verbs and shipped `completions` beside them as the shell-completion utility, and the guides never said so.

### Scope

- `cli/src/skill.rs` › `render_skill_line`: the plain line for a skill with no topics says so and names the command that prints its `SKILL.md`; the line count of the file does not grow (it sits at the file-size ratchet).
- `cli/tests/guide.rs`: the plain list and the `guide --help` text for a skill with no topics.
- `CLAUDE.md` and `README.md`: ten verbs plus the `completions` utility, as ADR 0060 decided.
- Kept: `guide --list --json` stays `{"name":"living-docs-drift","topics":[]}`; no verb, flag or JSON field changes.

### Decision

- **Say it in the line, keep the skill listed.** A skill with no topics renders as `<name>: no topics; living-docs guide <name> prints its SKILL.md`. Not chosen: hiding topic-less skills from the list, which would hide a skill `guide <name>` serves; or adding a `rules/` topic to the drift skill, which would duplicate its `SKILL.md` and leave the next topic-less skill with the same defect.

### Acceptance

- `living-docs guide --list --plain` prints the line `living-docs-drift: no topics; living-docs guide living-docs-drift prints its SKILL.md`, and every skill with topics prints as before.
- `living-docs guide --help --plain` carries that same text for the drift skill and no `living-docs-drift:;`.
- `living-docs guide --list --json` is byte-identical to before for every skill.
- `scripts/check-file-size.sh` passes with `cli/src/skill.rs` at or under its baseline.
- `CLAUDE.md` and `README.md` name ten verbs plus `completions`.
