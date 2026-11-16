---
type: Issue
title: A failing callout reconcile after a renumber leaves the bundle renamed
description: Fold the callout rewrite into the renumber rollback journal, or name the committed rename in the error
status: closed
timestamp: 2026-11-16T20:26:43Z
---

## 0145. A failing callout reconcile after a renumber leaves the bundle renamed

### Scope

`write set <TYPE/NNNN> number` and a renaming retitle now roll back every write and the rename on failure (issue 0130), but `settle_renamed` in `living-docs-core/src/commands/set.rs` runs `callout::reconcile` and the index refresh after `relocate` has committed. A write that fails inside the callout reconcile returns an error with the record already renamed. Found by the review of issue 0130 (reasoned, not reproduced).

### Decision

- Fold the callout rewrite into the relocate journal of issue 0130: `settle_renamed` drafts the callout reconcile before the apply phase, so a failing callout write undoes every write and the rename (owner, 2026-11-16). Not taken: only naming the committed rename in the error.

### Acceptance

- A renumber whose callout reconcile write fails leaves the bundle byte-identical, or exits naming the changed files; a test with a store that fails that write proves it.

### Outcome

- A renumber or renaming retitle drafts the moved record's callout before the apply phase (`callout::draft_reconciled`) and writes it inside the relocate journal, so a failing callout write, or a callout refused for an unwritable key, leaves the bundle byte-identical. A retitle that keeps its filename still reconciles in `settle_renamed`. Proof: `living-docs-core/src/commands/set/retitle/callout_rollback_tests.rs` (7 tests, the failure and keep-filename cases red first); `cargo test --workspace` green.
