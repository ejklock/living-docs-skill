---
type: ADR
title: The decision reminder is a non-blocking PostToolUse nudge, once per commit cycle
description: Replace the blocking Stop hook with a PostToolUse additionalContext nudge after Edit or Write, still opt-in and once per commit cycle
owner: Evaldo Klock
status: Accepted
supersedes: [0083, 0087]
timestamp: 2026-11-15T23:09:26Z
---

# 0089. The decision reminder is a non-blocking PostToolUse nudge, once per commit cycle

## Context

[ADR 0083](/adr/0083-an-opt-in-stop-hook-prompts-the-agent-to-record-a-material-choice-in-the-active-issue-before-the-turn-ends.md) shipped an opt-in end-of-turn prompt as a Claude Code Stop hook that printed `{"decision":"block"}`, and [ADR 0087](/adr/0087-the-decision-prompt-fires-at-most-once-per-commit-cycle.md) limited it to once per commit cycle. The owner asked that the hook nudge the agent instead of blocking anything.

Claude Code's hook reference (https://code.claude.com/docs/en/hooks.md, read 2026-11-15) leaves no non-blocking path at Stop: `decision: block` and Stop's `hookSpecificOutput.additionalContext` both continue the turn, plain stdout at Stop does not reach the model, and `systemMessage` is shown to the user, not the model. A PostToolUse hook, by contrast, can return `{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"..."}}`: the text is added to the model's context beside the tool result, nothing is blocked and no turn is forced, capped at 10,000 characters. PostToolUse fires after every successful tool call, so the once-per-commit-cycle state of ADR 0087 is still needed.

## Decision

We will deliver the decision reminder as a PostToolUse `additionalContext` nudge after a successful Edit or Write, at most once per commit cycle, replacing the Stop hook.

- **Trigger.** Opt-in through `LIVING_DOCS_DECISION_PROMPT=1`; the event is PostToolUse with matcher `Edit|Write`; the edited file lies outside the docs bundle; no path inside the bundle has changed in the working tree; and the hook has not yet nudged for the current `HEAD` (state under the git directory, as ADR 0087).
- **Output.** Exit 0 with the PostToolUse `additionalContext` JSON carrying the same reminder text (record a material choice in the active issue's Decision section; the command replaces the whole section; an ADR only when a future reader would pay to rediscover it; otherwise carry on). Never `decision`, never `continue: false`, never a Stop entry.
- **Install.** `manage install hooks` wires a PostToolUse entry with matcher `Edit|Write` instead of the Stop entry, and `manage uninstall` removes it; an existing Stop entry written by an earlier install is removed on install and uninstall.
- **Measure.** The 20-prompt kill criterion of ADR 0083 still applies, counted as nudges.

Rejected alternatives:
- **Keep the Stop hook** (ADR 0083 and 0087). It blocks the end of the turn, which the owner does not want.
- **UserPromptSubmit nudge.** It is non-blocking, but arrives at the start of the next turn, after the moment the rationale was in context.
- **Remove the hook.** It leaves only the session line, which the evidence in research 0005 says decays during a session.

## Consequences

**Easier / gained:**
- The agent is reminded at the moment it edits code, and nothing stops or extends its work.

**Harder / accepted trade-offs:**
- A nudge can be ignored; the kill criterion measures whether it works at all.
- The hook runs on every successful Edit or Write and must stay fast (about 120–140 ms measured on a 3,279-file repository).

**Follow-ups:**
- An issue to rewrite the script for the PostToolUse payload and output, change the install wiring, and update the constitution, procedure and help texts.

## Verification

**Implementation impact:** `skills/living-docs/hooks/decision-prompt.sh`, `cli/tests/decision_prompt.rs`, `cli/src/hooks.rs`, `cli/tests/hooks_install_stop.rs`, `cli/src/args/help.rs`, `cli/src/args/sub.rs`, `CONTRIBUTING.md`, `skills/living-docs/rules/procedure.md`, `docs/constitution.md`.

**Verification criteria:**
- With the opt-in set, an Edit of a code file outside the bundle produces one PostToolUse `additionalContext` JSON and no `decision` key; further edits on the same HEAD produce nothing; an edit inside the bundle produces nothing.
- `manage install hooks` writes a PostToolUse entry with matcher `Edit|Write` and no Stop entry; uninstall restores settings as before install.
