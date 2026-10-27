---
type: Issue
title: core keeps dead set and supersede run functions
description: Remove the uncalled set and supersede run functions, so the core maps no result to an exit code.
status: open
timestamp: 2026-10-27T02:38:49Z
---

## 0102. core keeps dead set and supersede run functions

### Scope

`living-docs-core` still keeps two `run` functions, `commands::set::run` and `commands::supersede::run`. Each one maps a result to a `std::process::ExitCode` and writes the error to stderr with `eprintln!`. Nothing in the workspace calls them: the CLI calls `set::apply` and `supersede::supersede` through its own wrappers in `cli/src/commands/set.rs` and `cli/src/commands/supersede.rs`. They are the last of the leftovers that issue 0101 removed.

### Decision

- Remove both functions and the `ExitCode` imports that only they use.
- The core then owns no command exit code: mapping a result to an exit code, and writing it to stderr, belong to the fronts.

### Acceptance

- `git grep -nE 'pub fn run\b' -- living-docs-core/src` lists nothing.
- `git grep -n 'eprintln!' -- living-docs-core/src/commands/set.rs living-docs-core/src/commands/supersede` lists nothing.
- The CLI's behavior is unchanged: the whole suite passes, and `read --json` and `check --json` stay byte-identical on the bench bundles.

### Outcome

`set::run` and `supersede::run` are gone from `living-docs-core`, along with the `ExitCode` imports that only they used. The core now has no `run` function: `git grep -nE 'pub fn run\b' -- living-docs-core/src` prints nothing, and neither file calls `eprintln!`. The docblocks of `set::apply` and `supersede::supersede` no longer point at the removed wrappers.

Verified:

- `cargo test --workspace` runs the same 1278 tests, with the same names, before and after. The change adds no test because nothing called either function.
- `read --json` and `check --json` match the parity corpus byte for byte on the 2500-record bundle.

Left as is: the docblock at `living-docs-core/src/check/mod.rs:121` links to a `run` that the check module no longer has. That link predates this change.
