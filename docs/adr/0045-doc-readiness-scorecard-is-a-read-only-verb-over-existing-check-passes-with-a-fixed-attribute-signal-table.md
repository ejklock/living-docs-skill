---
type: ADR
title: Doc-readiness scorecard is a read-only verb over existing check passes with a fixed attribute-signal table
description: scorecard is a read-only verb over existing check passes with a fixed attribute-signal table; grades per attribute, overall grade is the minimum, never gates
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-08-27T21:25:24Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0045. Doc-readiness scorecard is a read-only verb over existing check passes with a fixed attribute-signal table

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

<!-- The forces at play. What problem forced a decision? What constraints bound it?
     Written so a newcomer understands the pressure without prior knowledge. Link any
     research artifact or PRD/issue that motivates it, bundle-relative:
     [research](/research/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md). -->

Agents are the primary consumers of a living-docs tree, and a single pass/fail from `check` hides WHERE a corpus is weak ([issue 0037](/issues/0037-doc-readiness-scorecard-a-check-subreport-that-grades-a-docs-tree-from-human-era-to-agent-ready.md)). Readiness is capped by the weakest attribute, never averaged. Every signal must be deterministic and computable from the tree (plus, optionally, the projection); the tool grades structure, never prose quality (ADR 0001).

## Decision

We will add a read-only `living-docs scorecard` verb that reruns the existing check passes, aggregates their findings into a fixed attribute-signal table, and renders a table plus `--json`.

1. **Attributes and signals (the locked table):**
   - Trusted — doc-gate invariant violations (0 = pass); projection freshness: sync_meta fingerprint state, `not measured` when no projection exists.
   - Contextual — glossary/constitution presence per the doctype registry; index completeness (every record indexed).
   - Traceable — supersede chains intact; open MOVED-SOURCE advisories count.
   - Governed — owner coverage ratio over doc types whose registry row requires owner.
2. **Grades per attribute:** `agent-ready` (all signals clean), `in-transition` (advisory-level findings only or a partial ratio), `human-era` (invariant violations or a zero ratio), `not measured` (signal source absent). The overall grade is the MINIMUM across measured attributes — never an average.
3. **Read-only and informational:** the verb never mutates the tree and always exits zero on a conformant tree; grades never gate. Promotion of any signal to a gate is a separate decision.
4. **Reuse, not reimplementation:** signals come from the existing check passes and helpers (Reporter counts, owner requirement scan, moved-source pass, sync_meta reader). The scorecard adds aggregation and rendering only.

## Consequences

**Easier / gained:**
- The next docs investment is named by the weakest row instead of guessed.
- Zero new invariants: the scorecard cannot break CI.

**Harder / accepted trade-offs:**
- The verb reruns check passes, paying their cost twice when run alongside `check`. Acceptable at corpus scale.
- Grade thresholds are coarse (clean/advisory/violation); finer rubrics are future scope.

**Follow-ups:**
- Sweep the real corpus queue surfaced by the Traceable row (MOVED-SOURCE follow-up from ADR 0044).

## Verification

<!-- OPTIONAL — include when this decision must be honored in code, so the doc closes the
     doc → implement → verify loop an agent (and any review step) can consume. Omit
     for a purely advisory record. Keep criteria checkable, not aspirational.
     Implementation impact: files / modules this decision touches, e.g. `src/store.py`.
     Fitness function: the test / lint / arch-unit assertion that fails if the second
     verification criterion is violated (see `rules/adr-conventions.md` rule 6). -->

**Implementation impact:** `living-docs-core` scorecard module over the check subsystem, CLI `scorecard` verb (own module per the responsibility split), `--json` rendering.

**Verification criteria:**
- On a conformant fixture tree the verb exits zero and grades every measured attribute; absent signal sources render `not measured`, never an error.
- The same tree always produces the same scorecard (deterministic, idempotent); the overall grade equals the minimum measured attribute grade.
- Fitness function: a fixture test degrades one attribute at a time (an owner removed, a moved source introduced) and asserts only that attribute's grade drops and the overall grade follows the minimum.

# References

[1] [Making Your Data Ready for Agentic AI — Sadalage & Chandrasekaran](https://martinfowler.com/articles/making-data-ready-for-agentic-ai.html)
