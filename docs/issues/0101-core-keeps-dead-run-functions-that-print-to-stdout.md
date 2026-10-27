---
type: Issue
title: core keeps dead run functions that print to stdout
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-10-27T00:47:08Z
---

## 0101. core keeps dead run functions that print to stdout

### Scope

`living-docs-core` keeps five `run` functions that print to stdout with `print!`/`println!`: `commands::effective::run`, `commands::fmt::run`, `commands::index::run`, `commands::new::run` and `commands::next::run`. Nothing in the workspace calls them: the CLI has its own `run_*` wrappers, which write through its stdout writer. They are leftovers from before the hexagonal split, and they would panic on a closed pipe if anything ever called them.

### Decision

- Remove the five functions. Also remove any helper that only they use, such as the filesystem-based `next::next_number`, along with the tests that exist only for those helpers.
- The core then never writes to stdout; presentation belongs to the fronts.

### Acceptance

- `git grep -nE '(^|[^e])print(ln)?!' -- living-docs-core/src fs-store/src` lists nothing outside test code.
- The CLI's behavior is unchanged: the whole suite passes, and `read --json` and `check --json` stay byte-identical on the bench bundles.

### Outcome

The five uncalled `run` functions are gone from `living-docs-core`: `effective`, `fmt`, `index`, `new` and `next`. So are `next::next_number`, `highest_numeric_prefix` and `new::report_index_failure`, which only they used. The core no longer writes to stdout: `git grep -nE '(^|[^e])print(ln)?!' -- living-docs-core/src fs-store/src` prints nothing.

- **Rewritten tests:** the six `fmt` tests that called `fmt::run` now call `fmt::outcome` and still prove the mode. A check-only run must return `Outcome::Pending` and a writing run `Outcome::Rewritten`; swapping either one fails the core tests.
- **Deleted test:** one test is gone, the parity test between the filesystem allocator and the store allocator. `next_number_from_store` keeps its own tests.
- **CLI unchanged:** the whole suite passes, and `read --json` and `check --json` match the parity corpus byte for byte on the 2500-record bundle.

Left for later: `set::run` and `supersede::run` still return an `ExitCode` from the core. They print nothing.
