---
type: PRD
title: <Feature / capability name>
description: <One sentence — what capability this specifies.>
status: Draft
timestamp: <ISO 8601 datetime>
---

# NNNN. <Feature / capability name>

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Draft | Accepted | Implemented. `superseded_by` is absent by default;
     `living-docs supersede` sets Superseded on this record -- never by hand --
     when a later PRD replaces it. -->

## Problem / Motivation

<!-- Lead with the problem, not the solution. If you can only state it as a solution,
     grill it first to find the underlying need. -->

{{PROBLEM}}

## Goals

<!-- What success looks like, not a task. -->

- {{GOAL}}

## Non-goals

<!-- Name the tempting-but-excluded things. -->

- {{NON_GOAL}}

## Requirements

<!-- Functional requirements in EARS patterns (ADR 0035): ubiquitous "The system shall ...";
     event-driven "When {trigger}, the system shall ..."; state-driven "While {state}, the
     system shall ..."; unwanted behavior "If {condition}, then the system shall ...";
     optional "Where {feature}, the system shall ...". Each statement must be testable and
     carries a stable PRD-scoped ID (FR-1, FR-2, ...). IDs never renumber; a dropped
     requirement leaves a gap. BDRs cite the IDs they prove; `check` traces coverage once
     this PRD leaves Draft. -->

- **FR-1** — When {{TRIGGER}}, the system shall {{RESPONSE}}.
- **FR-2** — The system shall {{REQUIREMENT}}.

## Quality requirements (NFRs)

Non-functional requirements as **quality-attribute scenarios** (six-part: source →
stimulus → artifact → environment → response → response-measure), each bound to a
verifying instrument. A quality requirement without an instrument is a vibe.

<!-- e.g. Performance: a client issues a read to the API under 10x peak load, returns
     successfully in < 200 ms at p99, verified by a load test or CI floor. -->

| ID | Quality attribute | Scenario (source · stimulus · artifact · environment · response · measure) | Verified by |
|---|---|---|---|
| NFR-1 | {{QUALITY_ATTRIBUTE}} | {{SCENARIO}} | {{INSTRUMENT}} |
| NFR-2 | {{QUALITY_ATTRIBUTE}} | {{SCENARIO}} | {{INSTRUMENT}} |

<!-- Measure before committing to the complexity that meets the NFR; lock the measured
     floor in CI; record the decision + fitness function in an ADR. -->

## Acceptance criteria

<!-- An observable condition proving a requirement is met. -->

- {{ACCEPTANCE_CRITERION}}

## Success metrics

<!-- A quantified outcome that confirms the problem is solved after delivery -- not task
     completion, e.g. "Checkout abandonment rate drops by ≥10% within 30 days of
     launch." -->

- {{SUCCESS_METRIC}}

## Behavior (BDRs)

<!-- Link each BDR that specifies observable behavior this PRD defines or changes,
     bundle-relative: [BDR](/bdr/NNNN-<slug>.md). BDRs carry Mermaid diagrams, textual
     descriptions, and Given/When/Then scenarios. -->

- {{BDR_LINK}}

## Open questions

<!-- Each ideally headed toward an ADR (how/architecture) or a BDR (what the system
     must observably do). -->

- {{OPEN_QUESTION}}

## Decision log

<!-- Link to the ADR(s) and BDR(s) that resolved the open questions, once made. -->

- {{DECISION_LOG_ENTRY}}

## Related

- Constitution: [/constitution.md](/constitution.md)
- Issues: [/issues/NNNN-<slug>.md](/issues/NNNN-<slug>.md)
- Research: [/research/NNNN-<slug>.md](/research/NNNN-<slug>.md)
