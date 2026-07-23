---
type: Issue
title: <Issue title>
description: <One sentence — the change and its motivation.>
status: open
timestamp: <ISO 8601 datetime>
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## <Issue title>

<What the change is and why. If it implements a PRD or ADR, link it bundle-relative:
"Implements [ADR NNNN](/adr/NNNN-<slug>.md)" / "Part of [PRD NNNN](/prd/NNNN-<slug>.md)".>

### Scope

<What's included. For removals/refactors, state what is explicitly KEPT.>

### Acceptance

- <Observable, testable condition for "done".>

### Plan

<Short outline of the approach. For a large task, list the slices.>
