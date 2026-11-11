---
type: ADR
title: "Semantic refusal triggers become instruments or advisory: DIAGRAM and DUPLICATE checks, and no prose-only hard stops"
description: the three semantic refusal triggers (stale diagram, duplicate home, doc trail) become instrumented advisories or, for the doc trail, materiality-gated — check emits DIAGRAM and DUPLICATE lines and the enforcement-modes file keeps no prose-only hard stop; the agent runs check and treats advisories as work
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:54:56Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0055. Semantic refusal triggers become instruments or advisory, with no prose-only hard stops

## Context

`enforcement-modes.md` lists seven refusal triggers. Four (orphan, silent rewrite, untyped doc, broken link) are mechanical and run through `check`. Three are semantic with no oracle by the file's own admission: stale diagram (2), duplicate home (6), broken doc trail (7). Under `strict` all seven carried hard-stop weight. A hard stop with no instrument does not produce compliance; it produces over-production — an agent that cannot prove "no duplicate home" resolves the uncertainty by writing more. The skill's own stance is instrument-first; these three were the exception to it.

## Decision

We will build an instrument for each semantic trigger or demote it, keeping no prose-only hard stop:

- **Trigger 2, stale diagram → `DIAGRAM` instrument.** Mermaid node labels in `docs/architecture/*.md` are compared with a declared module list (`docs/architecture/diagram-scope.txt`, one name per line). A node with no module, or a module with no node, is an advisory `DIAGRAM` line. Absent scope file is a no-op (nothing to compare); promotion to a gate waits until the false-positive rate is measured.
- **Trigger 6, duplicate home → `DUPLICATE` instrument.** Near-duplicate detection over record bodies of the same type using shingled Jaccard similarity above a high threshold, on prose with headings and boilerplate labels stripped so the shared MADR skeleton does not inflate the score. Advisory `DUPLICATE` line naming both files.
- **Trigger 7, doc trail → advisory under materiality (ADR 0052).** The hard stop already applies only to *material* decisions; for everything else the trigger is advice, pointing at the check output.
- **Where the hard stop lives.** The pre-commit hook runs `check`; the agent prompt says "run `check`, fix what it prints", not seven paragraphs of refusal rules to adjudicate alone.

## Consequences

**Easier / gained:**
- Every semantic trigger now has a check behind it or a materiality gate; the refusal section shrinks to "run check; non-zero is blocked; advisories are work to schedule".

**Harder / accepted trade-offs:**
- Both new advisories under-report to stay trustworthy — `DIAGRAM` needs a declared scope, and `DUPLICATE` uses a high threshold — so a real duplicate can slip; a missed advisory beats an eroded one.

**Follow-ups:**
- Measure the false-positive rate on real corpora before promoting either advisory to a gate.

## Verification

**Implementation impact:** `living-docs-core/src/check/semantic/` (new: `mod.rs`, `diagram.rs`, `duplicate.rs`), `living-docs-core/src/check/mod.rs`, `skills/living-docs/rules/enforcement-modes.md`, `skills/living-docs/SKILL.md`.

**Verification criteria:**
- `check` prints `DIAGRAM` and `DUPLICATE` advisories on fixtures built for each and the exit code is unchanged; `check docs` over this repo stays green.
- `enforcement-modes.md` labels each trigger mechanical / instrumented-advisory / materiality-gated, with no prose-only hard stop, and the SKILL.md refusal section is reduced to the run-check rule.
