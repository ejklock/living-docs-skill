---
type: Issue
title: check skips every entry of a supersedes list that mixes record numbers and other values
description: check judges every record-number entry of a supersedes list that also holds other values, in both link directions, so a valid entry is no longer skipped.
status: open
timestamp: 2026-10-25T03:14:26Z
---

## 0095. check skips every entry of a supersedes list that mixes record numbers and other values

`check` leaves a mixed supersedes list such as `supersedes: [0001, abc]` entirely unjudged by the one-way link rule, valid entry included. The cause is that `extract_record` returns `supersedes = None` for a list that is not all record numbers (living-docs-core/src/record/canonical.rs › record_numbers), and the link check reads only the typed field. Reproduced in the review of the one-way link slice with the built binary: 0001 has no `superseded_by`, 0003 has `supersedes: [0001, abc]`, and `check` reports no link violation.

### Scope

- living-docs-core/src/check/supersede_links.rs.
- Whether a mixed list should be reported at all (fmt and set already treat it as untyped).

### Acceptance

- Decide first: either report the mixed list as its own violation, or judge each numeric entry of the raw sequence. Record the choice in this issue.
- A test pins the chosen behavior for `[0001, abc]`, and the existing link tests stay green.

### Plan

Read the raw `supersedes` sequence from the frontmatter tail in the link check, or add a separate rule for a non-numeric entry, as the decision says.

### Decision

The owner chose to judge each record-number entry of the raw `supersedes` sequence by the one-way link rule, in both link directions: a carrier's `supersedes: [0001, abc]` is judged for `0001`, and a target whose `supersedes` is `[0001, abc]` counts as naming `0001` back. An entry that is not a record number stays unjudged by this rule, as `manage fmt` and `write set` already treat it, and no new violation kind is added. The typed `supersedes` field stays `None` for a mixed list; the link check reads the raw sequence itself. Not taken: reporting the mixed list as its own violation; both; leaving it as is.
