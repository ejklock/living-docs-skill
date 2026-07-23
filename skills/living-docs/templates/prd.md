---
type: PRD
title: <Feature / capability name>
description: <One sentence — what capability this specifies.>
status: Draft
timestamp: <ISO 8601 datetime>
---

# NNNN. <Feature / capability name>

<!-- Status lives in frontmatter (`status`: Draft | Accepted | Implemented | Superseded),
     not a body line. `superseded_by` is absent by default; `living-docs supersede`
     adds it when a later PRD replaces this one. -->

## Problem / Motivation

<The user or system pain. Lead with the problem, not the solution. If you can only
state it as a solution, grill it first to find the underlying need.>

## Goals

- <Outcome 1 — what success looks like, not a task>

## Non-goals

- <What this explicitly does NOT cover. Name the tempting-but-excluded things.>

## Requirements

1. <Numbered, testable statement.>
2. <…>

## Quality requirements (NFRs)

Non-functional requirements as **quality-attribute scenarios** (six-part: source →
stimulus → artifact → environment → response → response-measure), each bound to a
verifying instrument. A quality requirement without an instrument is a vibe.

| Quality attribute | Scenario (source · stimulus · artifact · environment · response · measure) | Verified by |
|---|---|---|
| <e.g. Performance> | <e.g. A client · issues a read · to the API · under 10× peak load · returns successfully · in < 200 ms at p99> | <load test / CI floor / security check / inspection> |
| <e.g. Availability> | <…> | <…> |

<!-- Measure before committing to the complexity that meets the NFR; lock the measured
     floor in CI; record the decision + fitness function in an ADR. -->

## Acceptance criteria

- <Observable condition proving a requirement is met.>

## Success metrics

- <Quantified outcome that confirms the problem is solved after delivery — not task
  completion. E.g. "Checkout abandonment rate drops by ≥10% within 30 days of launch.">

## Behavior (BDRs)

- <Link each BDR that specifies observable behavior this PRD defines or changes,
  bundle-relative: [BDR](/bdr/NNNN-<slug>.md). BDRs carry Mermaid diagrams, textual
  descriptions, and Given/When/Then scenarios.>

## Open questions

- <Unresolved decision — each ideally headed toward an ADR (how/architecture) or a
  BDR (what the system must observably do).>

## Decision log

- <Link to the ADR(s) and BDR(s) that resolved the open questions, once made.>

## Related

- Constitution: [/constitution.md](/constitution.md)
- Issues: [/issues/NNNN-<slug>.md](/issues/NNNN-<slug>.md)
- Research: [/research/NNNN-<slug>.md](/research/NNNN-<slug>.md)
