---
type: Issue
title: check module docblock links to a run function that no longer exists
description: Fix the check module docblock that links to a run function the module no longer has.
status: open
timestamp: 2026-10-27T13:00:10Z
---

## 0104. check module docblock links to a run function that no longer exists

### Scope

The docblock above the plain violation-list entry point in `living-docs-core/src/check/mod.rs` (line 121) says "The invariants [`run`] validates". The check module has no `run` function, so the intra-doc link resolves to nothing and `cargo doc` reports it.

### Decision

- Point the docblock at the function that runs the full check today, or reword it so it names no symbol.
- Change no code.

### Acceptance

- `cargo doc --no-deps -p living-docs-core` reports no unresolved link at `check/mod.rs`.
