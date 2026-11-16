---
type: ADR
title: Reserved files are read and written through the rooted store port, so the bundle guard covers index.md and log.md
description: Route every content read and write of index.md and log.md through DocStore, widening the port contract from a doc record to any bundle file, so the FsStore rooted guard covers them; amends ADR 0073
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-16T00:18:18Z
---

# 0082. Reserved files are read and written through the rooted store port, so the bundle guard covers index.md and log.md

## Context

[ADR 0073](0073-the-filesystem-store-is-rooted-at-the-bundle-and-refuses-any-read-write-or-rename-that-resolves-outside-it.md) roots `FsStore` at the bundle, and the store refuses a read, write or rename that resolves outside it. That guard covers only what goes through the `DocStore` port. The reserved files `index.md` and `log.md` do not: the core still reads and writes them through `std::fs` (`commands/index.rs`, `commands/index/root.rs`, `check/graph.rs`). A type directory's `index.md` that is a symlink to a file outside the bundle is therefore still overwritten by `manage index`, `write new`, `write set` and `write supersede`. ADR 0073 lists this as its follow-up, and issue 0096 carries it.

The docblocks in `living-docs-core/src/store.rs` describe each port method as acting on a "doc record", and the code that reads `index.md` justifies `std::fs` by a db-store backend that no longer exists: the `.md` tree is the only backend (ADR 0059). Two places can hold the guard: the port, which already has it, or a second guard in the core.

## Decision

We will route every read and write of a reserved file's content (`index.md`, `log.md`) through the `DocStore` port, so the rooted guard of ADR 0073 covers it. This amends ADR 0073: its Decision stands, and its trade-off that leaves the reserved files unguarded is closed.

The `DocStore` trait keeps its four signatures. Its contract widens from a doc record to any file of the bundle: `read`, `write` and `rename` act on a record or a reserved file, and the refusal rule of ADR 0073 applies to both. `list` keeps its meaning; which listed files are records stays the core's judgment. The port docblocks say so, and the docblocks that justify `std::fs` by the removed db-store go away.

A refused index is never written, and the outside file's bytes stay unchanged. `manage index` exits 2 naming the path. `write new`, `write set` and `write supersede` keep their existing contract for an index failure: the record is written inside the bundle, the refused index is named on stderr, and the verb exits 1. `check` reports such an index as one violation naming the path, the same way it reports a refused record.

Existence and kind probes (`is_dir`, a metadata check for a regular file) stay in the core: they read no content and write nothing.

Rejected: a guard on the core side, next to the reserved-file `std::fs` calls. It leaves the port's contract as it is, but it duplicates the adapter's path resolution in the core, keeps filesystem I/O in a crate whose contract is the domain plus the port, and a new reserved-file writer has to remember to call it.

Rejected: leaving the reserved files outside the guard. It costs nothing now, but a symlinked `index.md` keeps a write path out of the bundle that every index-regenerating verb reaches.

## Consequences

**Easier / gained:**
- No verb writes outside the bundle through a reserved file, so the boundary of ADR 0073 holds for every file the tool writes.
- The core loses its content `std::fs` calls for `index.md` and `log.md`, and a future backend inherits the reserved files with the same refusal.

**Harder / accepted trade-offs:**
- The port's contract is wider: an adapter now stores reserved files as well as records.
- Creating the bundle-root `index.md` today opens it with create-new semantics; through the port it becomes a check for absence followed by a write, which a concurrent writer could race. The CLI is a single writer per bundle, so the gap is accepted.
- Every index-regenerating verb gains the exit-2 failure mode for a refused index.

## Verification

**Implementation impact:** `living-docs-core/src/store.rs` (the widened contract), `living-docs-core/src/commands/index.rs` and `living-docs-core/src/commands/index/root.rs` (reads and writes through the store), `living-docs-core/src/check/graph.rs` (index reads through the store and the refused-index violation), and the tests of `manage index`, `write new`, `write set`, `write supersede` and `check`.

**Verification criteria:**
- An `index.md` that is a symlink to a file outside the bundle makes `manage index` and every verb that rewrites an index exit 2, naming the path, with the outside file's bytes unchanged.
- An `index.md` symlinked to a file inside the bundle keeps working.
- `check` reports such an index as one violation naming the path.
- `living-docs-core` no longer reads or writes `index.md` or `log.md` content through `std::fs`.
