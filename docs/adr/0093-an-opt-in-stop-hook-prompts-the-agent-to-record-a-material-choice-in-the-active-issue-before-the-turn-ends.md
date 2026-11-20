---
type: ADR
title: An opt-in Stop hook prompts the agent to record a material choice in the active issue before the turn ends
description: Ship an inert-by-default end-of-turn decision prompt for Claude Code with a deterministic once-per-change-set trigger, pointing at the active issue's Decision section, never a gate, removed if it captures nothing
owner: Evaldo Klock
status: Superseded
superseded_by: 0089
timestamp: 2026-11-15T16:02:42Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0089](0089-the-decision-reminder-is-a-non-blocking-posttooluse-nudge-once-per-commit-cycle.md).

# 0093. An opt-in Stop hook prompts the agent to record a material choice in the active issue before the turn ends

## Context

A coding agent makes many choices while it implements, and most of the rationale for them exists only in its context at that moment. [Research 0005](/research/0005-engineering-documentation-in-ai-assisted-development-living-architecture-domain-language-and-rules-decision-capture-issue-waves-and-a-local-html-view.md) collects the evidence: rationale mined from commits afterwards recovers what changed but not why; instruction-following decays as a session grows, so a rule taught at session start fades by the time the choice is made; and most agent-authored pull requests change code without touching any documentation. Today living-docs teaches the authoring rule once, in the SessionStart line, and enforces only at commit, where `check` sees records but cannot see a decision that was never written.

[ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) is the counterweight. It deleted the PreToolUse write-gate because the agent negotiated with the block instead of changing behaviour, because the gate existed on one harness only, and because "a failing `check` in the same session" is what changes behaviour. The constitution lists a write-time hook as out of scope. Any new hook has to show why it is not that hook again.

The owner chose ([issue 0144](/issues/0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md)) an opt-in Stop hook that points at the active issue's Decision section, measured as an experiment. [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) bounds what it may ask for: materiality is the only authoring rule, and a hook that demands a record per change recreates the inflation that deleted the BDR type.

## Decision

We will ship an opt-in end-of-turn decision prompt as a Stop hook, installed by `manage install hooks` for Claude Code and inert unless the project opts in.

- **Opt-in.** The script exits 0 silently unless `LIVING_DOCS_DECISION_PROMPT=1`, the environment-variable pattern of [ADR 0081](/adr/0081-the-distributed-pre-commit-hook-is-the-only-copy-and-environment-variables-opt-it-into-changed-only-and-owner-checks.md).
- **Deterministic trigger.** At Stop, the hook blocks only when all hold: the working tree has changed paths outside the docs bundle; no path inside the bundle has changed; the harness did not report that a Stop hook already continued this turn; and the set of changed non-bundle paths with their content hashes differs from the last set it prompted for. That last state lives under the git directory, never in the tree, so the hook prompts once per distinct change set and never on every turn. *Superseded in part by [ADR 0087](/adr/0087-the-decision-prompt-fires-at-most-once-per-commit-cycle.md): the state key is the current HEAD, so the hook prompts at most once per commit cycle.*
- **What it asks.** One message: if the turn made a material choice, write it into the active issue's Decision section with `write set issue/NNNN section Decision --from -`, and only a choice a future reader would pay to rediscover gets an ADR; if it made none, say "no material decision" and stop. When the branch name carries an issue number, the message names that issue. It never asks for a new record per change.
- **Never a gate.** It does not block a write, a commit or CI. `check` stays the only gate. The constitution's enforcement line is amended to name the opt-in prompt next to the session-teaching hook.
- **Kill criterion.** After the first 20 prompts in a consumer project, count how many ended in a record edit that a reviewer judges a true capture. If none did, the hook is removed by a superseding ADR.

Rejected alternatives:
- **The session line alone.** It is the cheapest and ships anyway (issue 0144, S3), but it is taught once at t=0 and the evidence says compliance decays as the session grows.
- **A `Decision:` commit trailer checked at `commit-msg`.** It is deterministic and harness-neutral, but it is a second gate beside `check`, it taxes every commit, and `Decision: none` becomes the reflexive answer: the negotiation ADR 0059 observed, moved to commit time.
- **Blocking until a record changes.** It removes the escape and so forces a record per change, the inflation ADR 0057 removed.
- **An LLM judge at Stop that decides whether a decision was made.** It reads the diff with judgment, which ADR 0001 keeps outside the binary, costs a model call per turn, and gives answers that change with the wording; ADR 0075 keeps such judges advisory and offline.
- **A PreToolUse write-gate.** Deleted by ADR 0059 on observed behaviour.

## Consequences

**Easier / gained:**
- The prompt fires while the rationale is still in the agent's context, the one moment the research says it can be captured.
- It costs nothing when off and one short message per distinct change set when on; no model call, no new verb.

**Harder / accepted trade-offs:**
- It is a hook on one harness first, the property ADR 0059 held against the write-gate. Accepted because it never blocks work and is opt-in; Codex and Cursor equivalents are follow-ups only if the experiment pays.
- No controlled study shows that an end-of-turn prompt improves decision capture (research 0005). This is an experiment with a kill criterion, not a proven mechanism.
- The agent may answer "no material decision" by reflex. That answer is visible in the transcript and is exactly what the kill criterion counts.

**Follow-ups:**
- An issue to implement the script, its installation in `hooks.rs`, the constitution line and the measurement in one consumer project.

## Verification

**Implementation impact:** `skills/living-docs/hooks/` (a new Stop script), `cli/src/hooks.rs` (install and uninstall of the Stop entry), `cli/tests/` (hook fixtures), `docs/constitution.md` (the enforcement line), `skills/living-docs/rules/procedure.md`.

**Verification criteria:**
- Without `LIVING_DOCS_DECISION_PROMPT=1` the script prints nothing and exits 0 in every state.
- With it set, fixture repositories prove: code changed and no bundle change → one block with the message; a bundle file also changed → no block; the stop-hook-active flag set → no block; the same change set a second time → no block; a new change set → one block.
- `manage install hooks` then `manage uninstall` leaves `.claude/settings.json` as it was before install.
- `living-docs check` passes over this bundle with the amended constitution.
