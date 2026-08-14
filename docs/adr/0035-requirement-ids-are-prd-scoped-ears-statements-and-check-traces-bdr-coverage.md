---
type: ADR
title: Requirement IDs are PRD-scoped EARS statements and check traces BDR coverage
description: PRD requirements carry stable FR-N/NFR-N IDs written in EARS patterns; a deterministic check rule traces each ID of a non-Draft PRD to a BDR that links the PRD and cites the ID — advisory at Accepted, violation at Implemented.
status: Proposed
timestamp: 2026-08-14T07:37:50Z
---

# 0035. Requirement IDs are PRD-scoped EARS statements and check traces BDR coverage

## Context

A PRD's `## Requirements` section is numbered prose, and a BDR's scenarios and Test Design
matrix name what each row proves — but nothing ties a specific PRD requirement to the BDR
that covers it. Coverage is enforced only by reviewer judgment, and the gap opens exactly
when it matters: after a PRD is accepted and its BDRs are authored later.

Industry context (vendor-sourced, `[COI]`, cited as context, never as a causal gate for
this decision): spec-driven tooling has converged on EARS (Easy Approach to Requirements
Syntax) as the requirement notation authoring agents consume reliably — five patterns
(ubiquitous / event-driven / state-driven / unwanted-behavior / optional) that force a
trigger and a testable response [1][2].

Epistemic type: **Judgment** — requirement-identity shape and the severity ladder are
trade-offs with no experiment available here; no number is attached. External critic:
**pending** (real authoring usage is the critic that qualifies).

## Decision

We will give every PRD requirement a stable, PRD-scoped ID — `FR-N` for functional
requirements, `NFR-N` for quality requirements — written in EARS patterns; IDs never
renumber, and a dropped requirement leaves a gap. A BDR claims coverage of an ID by
linking the PRD bundle-relative and citing the ID in its body. `living-docs check` gains a
deterministic traceability rule: every ID defined in a PRD body must be cited by at least
one BDR that links that PRD — reported as an advisory at `Accepted` (BDRs are normally
authored after acceptance; the pre-commit gate must not block that window), as a violation
at `Implemented` (claiming implemented with uncovered requirements is drift), and not at
all at `Draft` or `Superseded`.

## Consequences

**Easier / gained:**
- Requirement → scenario → test is a checkable chain, not reviewer memory; drift surfaces at `check` time.
- IDs stay short in prose because scope rides on the PRD link the BDR template already mandates.
- Draft PRDs and ID-less legacy PRDs are untouched — the rule fires only on IDs that exist past Draft.

**Harder / accepted trade-offs (the declared sacrifice):**
- An ID is meaningful only through a PRD-linking BDR: a BDR that cites `FR-3` but omits the
  PRD link earns no coverage, and a bare `FR-3` elsewhere in the bundle is ambiguous by
  design. Steelman of the rejected alternative — globally unique IDs (`PRD-0007-FR-1`)
  would make every citation self-contained and grep-able bundle-wide; rejected because
  they duplicate identity the mandatory PRD link already carries and bloat every mention.
- Inferred (similarity-based) tracing was rejected outright: it crosses the determinism
  boundary (no heuristics inside the tool).
- An Accepted PRD can sit uncovered behind only an advisory — the accepted cost of not
  blocking the accept-then-author-BDRs sequence. Steelman of hard-fail-at-Accepted: no
  drift window at all; rejected because `.githooks/pre-commit` runs `check`, so the normal
  authoring sequence would be uncommittable mid-flow.

**Follow-ups:**
- `templates/prd.md` seeds EARS-shaped `FR-N` rows and an ID column in the NFR table;
  `templates/bdr.md` scenarios and Test Design rows cite the IDs they prove.
- `rules/prd-conventions.md`, `rules/bdr-conventions.md`, and `rules/check.md` document
  the notation and the trace rule.

## Verification

**Implementation impact:** `living-docs-core/src/check/traceability.rs` (new), registered
in `living-docs-core/src/check/mod.rs`; `skills/living-docs/templates/{prd,bdr}.md`.

**Verification criteria:**
- An `Implemented` PRD defining `FR-1` with no BDR both linking it and citing `FR-1` fails
  `check`; the same bundle at `Accepted` passes with an advisory; at `Draft`, silently.
- Fitness function: `check::traceability` unit tests pin all three severities plus the
  no-credit case (a BDR citing the ID without linking the PRD).

# References

[1] [EARS — Easy Approach to Requirements Syntax (Mavin et al., RE'09)](https://ieeexplore.ieee.org/document/5328509)
[2] [Kiro docs — specs with EARS acceptance criteria](https://kiro.dev/docs/specs/)
