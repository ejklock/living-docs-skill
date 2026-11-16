---
type: Issue
title: reserved files such as index.md are written outside the rooted store guard
description: Every read and write of an index.md goes through the rooted store, so an index that resolves outside the bundle is refused by every verb and reported once by check.
status: open
timestamp: 2026-10-25T03:32:21Z
---

## 0096. reserved files such as index.md are written outside the rooted store guard

ADR 0073 roots the filesystem store at the bundle, and the store refuses any record read, write or rename that resolves outside it. The reserved files are outside that guard: `index.md` and the other reserved files are still read and written through `std::fs` in the core (living-docs-core/src/commands/index.rs, living-docs-core/src/check/graph.rs), not through the port. A type directory's `index.md` that is a symlink to a file outside the bundle can therefore still be overwritten by `index`, `new`, `set` or `supersede`. The ADR lists this as its follow-up.

### Scope

- Every core write of a reserved file (`index.md`, `log.md`) and the reads that feed a write.
- living-docs-core/src/commands/index.rs and the verbs that regenerate an index.

### Acceptance

- An `index.md` that is a symlink to a file outside the bundle makes `manage index` exit 2 and `write new`, `write set` and `write supersede` exit 1 after writing the record, each naming the path, with the outside file's bytes unchanged.
- An `index.md` symlinked to a file inside the bundle keeps working.
- `check` reports such an index as one violation.
- The full suite, `make check` and `se-gates check` pass.

### Plan

Route reserved-file reads and writes through the store port, so the rooted guard covers them, or apply the same guard at the core's reserved-file write path. Pick one in an ADR amendment if the port's contract changes.

### Decision

The owner chose to route the reserved-file reads and writes (`index.md`, `log.md`) through `DocStore`, so the rooted `FsStore` guard covers them, as [ADR 0082](/adr/0082-reserved-files-are-read-and-written-through-the-rooted-store-port-so-the-bundle-guard-covers-index-md-and-log-md.md) records; it amends ADR 0073 and widens the port's contract from a doc record to any bundle file. Not taken: a guard on the core side; leaving it as is.

When `write new`, `write set` or `write supersede` meets a refused `index.md`, the owner chose to keep the existing index-failure contract: the record is written, the index is named on stderr, and the verb exits 1; `manage index` exits 2. Not taken: mapping the refusal to exit 2 after the record is written; refusing before the record is written.
