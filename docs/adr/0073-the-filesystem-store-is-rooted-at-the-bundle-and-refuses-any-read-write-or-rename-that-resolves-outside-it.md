---
type: ADR
title: The filesystem store is rooted at the bundle and refuses any read, write or rename that resolves outside it
description: FsStore takes the bundle at construction and refuses a read, write or rename whose resolved path leaves it, moving the symlink guard out of the core
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-25T00:33:11Z
---

# 0073. The filesystem store is rooted at the bundle and refuses any read, write or rename that resolves outside it

## Context

A record reference can resolve to a file the bundle does not own. A record file that is a symlink to a path outside `docs/` is listed by the store like any other record, so `set`, `supersede` and `fmt` write straight through it into a file elsewhere on disk. Issue 0063's `set number` slice closed one entrance: `find_record` refuses a symlink that resolves outside the bundle. That fix carries two costs the review named. It is filesystem I/O (`symlink_metadata`, `canonicalize`) inside `living-docs-core`, whose contract (ADR 0059's layout) is the domain plus the `DocStore` port with no I/O. And it guards only the verbs that go through `find_record`: `fmt` over a bundle, `index` and the architecture views never ask it.

`FsStore` holds no state today. Every call is rooted at the absolute path it receives, so the adapter cannot tell a path inside the bundle from one outside it. The port itself (`list`, `read`, `write`, `rename`) carries no root either.

## Decision

We will root `FsStore` at the bundle. The constructor takes the bundle directory (`FsStore::rooted(bundle)`), and the stateless `FsStore::new()` goes away. The CLI front builds one store per verb from the bundle that verb works on (`--docs-dir`, or the bundle `check` and `fmt` resolve from their argument).

`read`, `write` and `rename` (both ends) refuse a path whose resolved location lies outside the canonical bundle root, with an `io::Error` naming the path and the bundle. Resolution follows symlinks on every existing component of the path. A path that does not exist yet (a new record, a rename target) is judged by its nearest existing ancestor, and the check runs before any directory is created. `list` keeps enumerating the bundle as it does today. The check that sits in `find_record` moves into the adapter, so the core loses its `std::fs` calls for it.

`check` reports a listed record the store refuses to read as its own violation, naming the path and the rule that a record must lie inside the bundle. It never reports that record as an empty or frontmatter-less file.

The `DocStore` trait keeps its four signatures. Its documentation gains the rule that an adapter may refuse a path outside the root it was built for, so a future backend is held to the same boundary without a new method.

Rejected: a port method `DocStore::confines(root, path)` with a permissive default that `FsStore` overrides. It moves the I/O out of the core with the smallest blast radius, but it keeps the guard opt-in per call site. Coverage stays exactly where `find_record` is, and every new verb has to remember to ask.

Rejected: keeping the check in the core next to the existing `std::fs` uses in `index.rs` and `frontmatter.rs`. It is the cheapest option, but it leaves the I/O rule broken and the other writers unguarded.

## Consequences

**Easier / gained:**
- No verb that writes through the store can reach a file outside the bundle, whether through a symlinked record, a symlinked type directory or a rename target.
- The core's record lookup is pure again: `find_record` resolves names, and the adapter owns the filesystem boundary.
- A future backend inherits the boundary from the port's documented contract.

**Harder / accepted trade-offs:**
- Every CLI verb now has a new failure mode: a path outside the bundle fails the read or write, with exit 2.
- The store needs the bundle at construction, so each verb builds it after resolving its bundle, and every `FsStore::new()` call site and test changes.
- `index.md` and the other reserved files are still read and written through `std::fs` in the core (`commands/index.rs`, `check::graph`), outside the port. This decision does not guard them.

**Follow-ups:**
- Route the reserved-file writes (`index.md`) through a rooted path guard too, tracked as its own issue.

## Verification

**Implementation impact:** `fs-store/src/lib.rs` (the rooted constructor and the guard), `living-docs-core/src/store.rs` (the documented rule), `living-docs-core/src/commands/reference/lookup.rs` (the core check removed), `living-docs-core/src/check/records.rs` (the refused-record violation), `cli/src/store.rs` and every CLI verb that builds a store, and the fs-store tests.

**Verification criteria:**
- `set`, `supersede` and `fmt` on a record that is a symlink to a file outside the bundle exit 2, name the path, and leave the target's bytes unchanged.
- A write through a symlinked type directory that points outside the bundle is refused before any directory is created.
- A symlink that resolves inside the bundle, and a bundle reached through a symlinked `--docs-dir`, keep working.
- `check` on a bundle holding a record linked outside it reports one violation naming that path, and no frontmatter finding for it.
- `living-docs-core` no longer calls `std::fs` to resolve a record reference.
