---
type: ADR
title: Hand-written record frontmatter is blocked at write time, detected by check, and taught at point of use
description: Enforce the CLI-owns-the-mechanics rule with three deterministic layers instead of advisory prose — a write-time hook, a canonical round-trip check, and point-of-use teaching.
status: Proposed
tags: [check, cli, enforcement, frontmatter, hooks]
timestamp: 2026-07-23T15:44:02Z
---

# 0019. Hand-written record frontmatter is blocked at write time, detected by check, and taught at point of use

## Context

The CLI-owns-the-mechanics rule ("use the verb for every deterministic step; hand-writing
frontmatter, hand-numbering, hand-maintaining an index is a process error") exists in two
places — the `procedure` topic and rule 10 of the hard-rules template — yet agents keep
authoring whole record files by hand, frontmatter included. Two structural gaps explain it:

1. **Disclosure gap.** Since ADR 0017 the SKILL.md stub is a pure router; the hard rule
   lives behind `living-docs skill --topic procedure`, which an agent may never load
   before writing. Nothing surfaces the rule at Write time.
2. **Enforcement gap.** The only deterministic gate today is the enforcer plugin's
   commit-boundary hook (`block-docs-drift.sh` → lint ratchet). It fires after the file
   was already hand-written, and a well-formed hand-written record passes it — the lint
   has no oracle for "this doc did not come from `new`".

Instructions never block; only gates block. Whether a Write bypasses the CLI is a fully
deterministic question, so it deserves a gate, not prose.

A boundary discovered while dogfooding: `new` scaffolds placeholders the author must
fill. Frontmatter keys therefore split into **CLI-owned** (`type`, `status`,
`supersedes`, `superseded_by`, `timestamp`, and numbering, which is path-derived per
ADR 0007) and **author-owned** (`description`, `tags` — judgment values). `title` is
deterministic from the `new` argument and moves to the CLI-owned side by making `new`
fill it.

## Decision

We will enforce CLI-owned authoring with three deterministic layers:

1. **Write-time hook (Claude Code, enforcer plugin).** A new PreToolUse hook on
   `Write`/`Edit` in `living-docs-enforcer` blocks, inside a docs bundle:
   a Write that creates a new `NNNN-*.md` record ("use `living-docs new`"); an
   Edit/Write whose change overlaps the frontmatter block **and** touches a CLI-owned
   key ("use `living-docs status`/`supersede`"); any direct write to a record
   directory's `index.md` ("run `living-docs index`"). Author-owned keys and the body
   stay freely editable. Same `LIVING_DOCS_ENFORCE=block|warn` knob, fail-open on any
   ambiguity, mirroring the existing hook's defensive contract.
2. **Canonical round-trip check (any harness).** The canonical record model
   (`ExtractedRecord`, `extract_record`, `to_canonical_markdown` — the ADR 0007
   byte-stable contract) moves from `db-store` into `living-docs-core` (domain, not
   adapter; `db-store` re-exports). `living-docs check` gains a violation: a record
   whose frontmatter block does not byte-equal its canonical re-serialization is
   flagged as hand-written. A new deterministic verb `living-docs fmt` canonicalizes
   frontmatter in place and is the remediation path the violation message names.
   The check verifies canonical **form** (key order, spacing, quoting), never values —
   author-owned values round-trip untouched.
3. **Point-of-use teaching.** `new` fills `title:` from its argument and ends its
   output with the body-only instruction ("write ONLY the body below the closing
   `---`; frontmatter and indexes are CLI-owned — `status`, `supersede`, `index`").
   The same single line enters the root `--help` and the SKILL.md stub — a bounded,
   one-line exception to the ADR 0017 pure-router rule, justified because it is
   precisely the rule the router cannot deliver in time.

## Consequences

**Easier / gained:**
- Bypassing the CLI in Claude Code fails fast, at the tool call, with the correct verb
  named in the block message — the agent self-corrects without burning a review round.
- Hand-written frontmatter is detectable in every harness and in CI via `check`, with a
  one-command deterministic fix (`fmt`).
- The canonical record model lands in the crate that owns the domain, unblocking any
  future front from reusing it without depending on the SQL adapter.

**Harder / accepted trade-offs:**
- A hand-written record that is byte-canonical passes layer 2 undetected; only layer 1
  catches it, and only under Claude Code. Accepted: the residue is harmless by
  construction (byte-canonical means indistinguishable from CLI output).
- The hook adds per-Write latency and one more moving part in the plugin; mitigated by
  the fail-open contract.
- One prose line returns to the SKILL.md stub, slightly softening ADR 0017's purity.

**Rejected alternatives:**
- **Commit-time-only enforcement (status quo).** Too late and blind to well-formed
  hand-writes.
- **A body-editing verb / blocking all direct edits.** Contradicts the settled
  procedure decision — wrapping prose edits in the CLI adds no determinism.
- **Duplicating the serializer in `living-docs-core`.** Two homes for the canonical
  contract would drift; the move keeps one home.

**Follow-ups:**
- Enforcer plugin release with the new hook (version bump in `ai-configs`).

## Verification

**Implementation impact:** `living-docs-core/src/record.rs` (new home),
`living-docs-core/src/check/`, `living-docs-core/src/commands/{new,fmt}.rs`,
`cli/src/main.rs`, `db-store/src/{record,serialize}.rs` (re-export shims),
`skills/living-docs/SKILL.md`, `ai-configs/plugins/living-docs-enforcer/hooks/`.

**Verification criteria:**
- A record whose frontmatter deviates from canonical serialization fails
  `living-docs check`; after `living-docs fmt` the same bundle passes, and `fmt` is
  idempotent (second run is a no-op).
- `new` output names the body-only rule and fills `title:`; round-tripping a fresh
  `new` scaffold through `extract_record` → `to_canonical_markdown` is a fixed point.
- Hook fixture tests: creating a record via `Write` is blocked; editing a CLI-owned
  key is blocked; editing body/`description`/`tags` is allowed; a lone `status:` line
  inside a body code fence is not a false positive.
