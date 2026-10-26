---
type: Issue
title: reserved files such as index.md are written outside the rooted store guard
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T03:32:21Z
---

## 0096. reserved files such as index.md are written outside the rooted store guard

ADR 0073 roots the filesystem store at the bundle, and the store refuses any record read, write or rename that resolves outside it. The reserved files are outside that guard: `index.md` and the other reserved files are still read and written through `std::fs` in the core (living-docs-core/src/commands/index.rs, living-docs-core/src/check/graph.rs), not through the port. A type directory's `index.md` that is a symlink to a file outside the bundle can therefore still be overwritten by `index`, `new`, `set` or `supersede`. The ADR lists this as its follow-up.

### Scope

- Every core write of a reserved file (`index.md`, `log.md`) and the reads that feed a write.
- living-docs-core/src/commands/index.rs and the verbs that regenerate an index.

### Acceptance

- An `index.md` that is a symlink to a file outside the bundle makes `index` (and every verb that rewrites an index) exit 2, naming the path, with the outside file's bytes unchanged.
- An `index.md` symlinked to a file inside the bundle keeps working.
- `check` reports such an index as one violation.
- The full suite, `make check` and `se-gates check` pass.

### Plan

Route reserved-file reads and writes through the store port, so the rooted guard covers them, or apply the same guard at the core's reserved-file write path. Pick one in an ADR amendment if the port's contract changes.
