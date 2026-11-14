---
type: ADR
title: stale-proposed counts only an issue that declares it implements the ADR
description: The stale-proposed advisory counts an issue toward an ADR only when the issue body declares Implements with a link to that ADR; a link from the ADR to a closed issue is history and no longer counts.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-12T16:13:09Z
---

# 0079. stale-proposed counts only an issue that declares it implements the ADR

## Context

`check` keeps one liveness advisory, `stale-proposed`, which [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) retained because it has a real oracle: an ADR still at `Proposed` whose implementing issue is already closed has a decision that landed while the record never left its birth state.

The oracle is approximated today by any link. `links_terminal_issue` in `living-docs-core/src/check/liveness.rs` flags a `Proposed` ADR whose body links any issue that is closed or superseded. An ADR's Context routinely cites a closed issue as history ("issue 0059 implemented the section key"), so a new decision that cites old work reads as stale the moment it is written. The author's only escape is to drop the citation, which removes the trail the record exists to keep.

The relation the advisory needs is already written down by the other side. The issue template's summary slot asks for `Implements [ADR NNNN](/adr/NNNN-slug.md)`, and 26 issues in this bundle carry it. That sentence is the issue declaring which decision its work lands; a citation in the ADR declares nothing.

## Decision

We will count an issue toward `stale-proposed` only when the issue's body declares that it implements the ADR: the word `Implements` (any case) followed directly by a markdown link that resolves to that ADR. A `Proposed` ADR is stale when at least one issue declaring it is closed or superseded. A link from the ADR to an issue no longer counts, and an issue that links the ADR any other way does not count either.

Rejected alternatives:

- **Keep the rule and document it.** Authors would be told not to link closed issues from a Proposed ADR. That trades a false advisory for a missing citation, and the rule would keep firing for every author who did not read the guide.
- **Require both directions: the ADR links the issue and the issue declares `Implements`.** It removes the same false positives but misses an ADR that never links its issue back, which is the common case in this bundle; the declaration on the issue alone is the stronger oracle.
- **A frontmatter key on the issue (`implements: adr/NNNN`).** Machine-precise, but a new CLI-owned field, a migration of every issue and a second place to say what the summary sentence already says.

## Consequences

**Easier / gained:**
- An ADR may cite closed issues as history without an advisory.
- An ADR whose implementing issue closed gets the advisory even when the ADR never linked the issue.

**Harder / accepted trade-offs:**
- An issue that implements an ADR without the `Implements [ADR NNNN](...)` sentence no longer feeds the advisory; 86 issues here phrase it otherwise, most of them closed against ADRs that are already settled.
- The check now reads every issue body for the declaration instead of only the links of each Proposed ADR.

**Follow-ups:**
- An issue implementing the check, with the deciding cases below as its acceptance.

## Verification

**Implementation impact:** `living-docs-core/src/check/liveness.rs` and its tests; the `check` guide topic where it describes `stale-proposed`.

**Verification criteria:**
- A Proposed ADR whose body links a closed issue that does not declare `Implements` it: no advisory.
- A Proposed ADR that links nothing, with a closed issue whose body holds `Implements [ADR NNNN](/adr/NNNN-slug.md)` for it: one `stale-proposed` advisory.
- The same issue open: no advisory. The same ADR Accepted: no advisory.
- `implements [ADR NNNN](...)` in lower case counts; `Implements ADR NNNN` with no link does not; an `Implements` link to another ADR does not.
- The liveness unit tests fail when the ADR-side link is counted again.
