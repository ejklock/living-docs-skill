---
type: ADR
title: "why <path>: a reverse index from Implementation-impact lists so provenance is a query, not a code comment"
description: living-docs why <path> inverts the Verification block's Implementation-impact lists (exact, directory-prefix, glob) over active records so an agent answers which records govern a file by query — exact match first — and never writes an ADR/BDR citation into a code comment
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:19:04Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0051. why &lt;path&gt;: a reverse index from Implementation-impact lists so provenance is a query, not a code comment

## Context

Consumers of this skill ban doc-trail citations in code comments (a comment never cites an ADR/BDR/PRD/issue number) — and the ban is right, because comments drift and lie. But the agent keeps writing them: one strict-mode consumer has 357 comments in `src/` citing an ADR, and the docblock/citation category was 44% of one audited review's required changes. The agent cites because the corpus dominates its context and it has **no other place** to keep the link code → decision. That link already exists on the doc side — `## Verification` → `Implementation impact: files / modules this decision touches` — it is simply never inverted.

## Decision

We will add a `living-docs why <path>` verb that answers "which records govern this file" by inverting the Implementation-impact lists over active records (superseded/deprecated excluded; stale per ADR 0049 excluded unless `--include-stale`):

- **Entry forms.** Each impact entry matches a query path as **exact** (`src/store.rs`), **directory prefix** (`src/`, or a bare directory), or **glob** (`src/**`, `src/*.rs`).
- **Ranked most-specific first.** A record matching by exact path outranks one matching by prefix, which outranks a glob — so `why src/store.rs` over an exact-`src/store.rs` record and a `src/**` record returns both, the exact match first.
- **Output.** Record id, title, status, and the record's `Verification criteria`, contracts first.
- **`--from-diff <range>`.** The CLI front resolves `git diff --name-only <range>` (as `brief` does, keeping core I/O-free) and passes the touched paths; `why` lists every record any of them touches, exiting 0 with no output when none match — the pre-emit check a Coder or a PreToolUse hook runs before claiming a task done.

The skill gains the rule: **provenance is a query, never a comment.** When an agent feels the need to cite a record in code, it runs `why` and cites nothing; a hook matching `(ADR|BDR|PRD|issue) \d{4}` in a diff is the instrument, so the prompt line becomes unnecessary.

## Consequences

**Easier / gained:**
- The single most frequent review finding (citation in code) loses its motive: the link lives in a query, not a comment that rots.

**Harder / accepted trade-offs:**
- The reverse index is only as good as the Implementation-impact lists; a record that never named its files is invisible to `why`. Accepted — the same block already feeds stale-impact (ADR 0049), so keeping it current earns two instruments.

**Follow-ups:**
- Ship the PreToolUse hook example that runs `why --from-diff` and blocks a diff introducing a record citation in a comment.

## Verification

**Implementation impact:** `living-docs-core/src/impact.rs` (new, shared with ADR 0049), `living-docs-core/src/commands/why.rs` (new), `cli/src/commands/why.rs` (new), `cli/src/args.rs` (`why` subcommand), `skills/living-docs/rules/citation-conventions.md`.

**Verification criteria:**
- Fixture: two ADRs with Implementation impact `src/store.rs` and `src/**`; `why src/store.rs` returns both, the exact match first.
- `why --from-diff <range>` lists records for every touched path and exits 0 with an empty list when none matches; superseded records never appear, stale only under `--include-stale`.
