---
type: ADR
title: A stale type index is in scope of a changed record of its type
description: check --changed-files reports index drift when the changed files name the index or any record of that type, and the hook passes staged deletions; amends the changed-only verdict of ADR 0081.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-20T19:58:23Z
---

# 0097. A stale type index is in scope of a changed record of its type

## Context

[ADR 0081](0081-the-distributed-pre-commit-hook-is-the-only-copy-and-environment-variables-opt-it-into-changed-only-and-owner-checks.md) lets a project opt the pre-commit hook into a changed-only verdict: `check --changed-files` reports only findings anchored at the staged records, so a bundle with older debt can still arm the gate. [ADR 0070](0070-type-indexes-merge-with-the-git-union-driver-and-check-fails-on-an-index-that-differs-from-its-regeneration.md) made a type index that differs from its regeneration a violation, anchored at the index file alone, and a test pinned that naming a record did not bring its index into scope.

The two together left a hole. A commit that stages a record and leaves that type's index stale passed the changed-only hook, because the stale file was the index and the index was not staged; a staged deletion was not passed to the check at all. On 2026-11-20 this was reproduced with a hand-set status on one ADR: the hook printed OK. An index is derived from its records, so a stale index beside a changed record is that change's own debt far more often than someone else's.

## Decision

We will treat a stale type index as in scope of any changed record of that type: `check --changed-files` reports the index-drift finding when the changed files name the index or any record in that type's directory. The distributed hook also passes each staged deletion as its directory's index.

Changed-only otherwise keeps its meaning. A finding in a record that is not among the changed files stays out of scope.

Rejected alternatives:

- **Keep the earlier contract and only pass staged deletions.** A hand edit that changes what the index shows still lands with the index stale, and only a full check in CI catches it.
- **Report only the drift the commit itself caused, by comparing with what the previous commit would generate.** It is exact, but `check` would depend on git history and on the commit being the unit of work.
- **Report every drift regardless of the changed files.** It turns changed-only back into the full gate, which is what ADR 0081 exists to avoid.

## Consequences

**Easier / gained:**
- A commit cannot leave the index of the type it touched stale and pass the changed-only hook.
- The repair is one command the finding names, `living-docs manage index <type>`, and it needs no judgment.

**Harder / accepted trade-offs:**
- In a bundle whose index was already stale, the first commit that touches a record of that type fails until the index is regenerated, although that commit did not cause the drift. The tool cannot tell the two apart without git history.
- A deletion inside a nested directory is scoped to that directory's index, not to the type's.

**Fitness function:** tests assert that a changed record of a type brings that type's stale index into scope, that a record of another type does not, and that the hook fails a commit staging a body edit, an addition or a deletion with the index left stale.
