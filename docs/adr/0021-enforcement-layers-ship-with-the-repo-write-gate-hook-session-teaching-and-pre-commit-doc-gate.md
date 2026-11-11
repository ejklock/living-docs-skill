---
type: ADR
title: "Enforcement layers ship with the repo: write-gate hook, session teaching, and pre-commit doc-gate"
description: Ship the ADR 0019 enforcement layers inside this repository — a versioned write-gate hook in the skill bundle wired via .claude/settings.json, a SessionStart hook that injects the body-only rule at t=0, and a pre-commit doc-gate — instead of depending on an externally installed enforcer plugin.
owner: Evaldo Klock
status: Deprecated
tags: [check, cli, enforcement, hooks, tokens]
timestamp: 2026-07-29T01:51:15Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0021. Enforcement layers ship with the repo: write-gate hook, session teaching, and pre-commit doc-gate

## Context

ADR 0019 established three enforcement layers for the CLI-owns-the-mechanics rule, and
ADR 0020 scoped the write-time layer to the four CLI-owned type directories. Two of the
three layers (the canonical round-trip in `check`/`fmt`, the point-of-use teaching in
`new` and `--help`) live in this repository. The decisive layer — the PreToolUse hook
that blocks a hand-write *before* the tokens are spent — lives only in the
`living-docs-enforcer` plugin in the external `ai-configs` repository.

Dogfooding shows the consequence: any session that has not installed that plugin
(fresh clones, web agents, CI agents, other harnesses) runs with zero write-time
enforcement, and agents keep authoring whole records by hand. The two in-repo layers
fire post-hoc: by the time `check` flags a hand-written record, the tokens for the
frontmatter, the numbering, and the index row were already spent. The rule's delivery
is also probabilistic — it sits behind a skill trigger plus a `--topic` load — while
the agent's decision to Write happens before either.

The constraint from ADR 0019 stands: instructions never block; only gates block.
The gap is distribution, not design — the gate must travel with the repository.

## Decision

We will ship the enforcement layers inside this repository, harness-wired at the repo
boundary:

1. **Write-gate hook in the skill bundle.** `block-docs-handwrite.sh` lives at
   `skills/living-docs/hooks/` — versioned with the skill, testable in this repo's CI —
   and `.claude/settings.json` wires it as a PreToolUse hook on `Write|Edit|MultiEdit`.
   It enforces the three ADR 0019 block rules under the ADR 0020 directory scope
   (`adr|bdr|prd|issues` under the bundle), honors `LIVING_DOCS_ENFORCE=block|warn`
   (default `block`), and fails open on any ambiguity.
2. **Session teaching hook.** `session-context.sh` (same bundle directory) runs on
   SessionStart: it emits the body-only rule plus the resolved CLI binary path into the
   session context unconditionally — closing the disclosure gap deterministically at
   t=0 instead of probabilistically behind a skill trigger — and points `core.hooksPath`
   at `.githooks/` when that directory exists.
3. **Pre-commit doc-gate.** `.githooks/pre-commit` runs `living-docs check <bundle>`
   with the resolved binary, so a hand-written record that slipped past the write gate
   fails in the same session it was authored, not at CI. When no binary is resolvable
   the gate skips with a notice (fail-open; CI still gates).

The external enforcer plugin remains valid for consumers; this decision makes the repo
self-enforcing without it. The hard-rules template gains the wiring snippet so consumer
projects can adopt the same layers.

## Consequences

**Easier / gained:**
- A hand-write inside a CLI-owned directory fails at the tool call in any Claude Code
  session of this repo, with the correct verb named — no tokens spent on frontmatter,
  numbering, or index rows the CLI generates for free.
- The body-only rule reaches every session deterministically via SessionStart, instead
  of depending on skill triggering.
- The hook is versioned and tested alongside the skill it enforces, in one CI.

**Harder / accepted trade-offs:**
- The block rules now exist in two places (this repo and the enforcer plugin); the
  plugin remains the distribution for non-repo contexts. Accepted: both mirror the same
  ADR 0019/0020 contract, and this repo's fixture tests pin the behavior.
- `.claude/settings.json` binds this repo to Claude Code hook semantics; other
  harnesses rely on the pre-commit and CI gates only. Accepted: fail-open by design.
- Sessions started before this change do not pick up the hooks (hook wiring snapshots
  at session start).

**Follow-ups:**
- A `living-docs hooks install` verb so consumer projects wire the same layers through
  the CLI instead of copying files by hand.

## Verification

**Implementation impact:** `skills/living-docs/hooks/{block-docs-handwrite.sh,session-context.sh}`,
`.claude/settings.json`, `.githooks/pre-commit`,
`skills/living-docs/tests/hooks/test-block-docs-handwrite.sh`, `Makefile` (`test-hooks`
target inside `check`), `skills/living-docs/templates/claude-hard-rules.md` (rule 10).

**Verification criteria:**
- A `Write` creating `docs/adr/NNNN-*.md`, a write to `docs/adr/index.md`, and an
  `Edit` touching a CLI-owned frontmatter key in a record each exit 2 naming the
  correct verb; body, `description`, and `tags` edits exit 0; a `status:` line inside
  a body code fence exits 0; paths under `docs/research/` and the bundle-root
  `docs/index.md` exit 0 (ADR 0020 scope).
- `LIVING_DOCS_ENFORCE=warn` turns every block into a stderr notice with exit 0; a
  missing `jq` or unparsable payload exits 0 (fail-open).
- Fitness function: `make test-hooks` (run by `make check`) passes all fixture
  sections.
