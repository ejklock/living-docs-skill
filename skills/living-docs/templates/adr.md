---
type: ADR
title: <Short decision title>
description: <One sentence — the decision and its scope.>
status: Proposed            # Proposed | Accepted | Superseded | Deprecated
supersedes:                 # NNNN of the ADR this replaces, if any
superseded_by:              # NNNN, set when a later ADR replaces this one
tags: []
timestamp: <ISO 8601 datetime, e.g. 2026-06-13T00:00:00Z>
---

# NNNN. <Short decision title>

<!-- Status lives in frontmatter (`status`), not a body line. When superseding a
     prior ADR, set `supersedes` here and `superseded_by` on the old record. -->

## Context

<The forces at play. What problem forced a decision? What constraints bound it?
Written so a newcomer understands the pressure without prior knowledge. Link any
research artifact or PRD/issue that motivates it, bundle-relative:
[research](/research/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md).>

## Decision

We will <the choice, in active voice — specific and testable>.

## Consequences

**Easier / gained:**
- <what this unlocks>

**Harder / accepted trade-offs:**
- <what this costs or forbids>

**Follow-ups:**
- <issues spawned, ADRs this may later require>

# References

<!-- Optional (OKF §8). External sources backing claims in Context. -->
[1] [<source>](<url>)
