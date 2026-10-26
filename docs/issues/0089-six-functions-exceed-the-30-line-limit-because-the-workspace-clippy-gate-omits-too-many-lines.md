---
type: Issue
title: six functions exceed the 30-line limit because the workspace clippy gate omits too_many_lines
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-10-25T02:00:40Z
---

## 0089. six functions exceed the 30-line limit because the workspace clippy gate omits too_many_lines

Found while running clippy with `-W clippy::too_many_lines` (the form `make cli-clippy` uses) during the usage-audit work (2026-10-24): six functions already on the branch exceed the 30-line limit CLAUDE.md sets.

- `supersede` in living-docs-core/src/commands/supersede/mod.rs (41 lines).
- `append_unreached` in living-docs-core/src/commands/index/root.rs (32 lines).
- `run_set` in cli/src/commands/set.rs.
- `crlf_adr_writes_equal_the_lf_twin_with_crlf` in cli/tests/set_read_back/crlf.rs (33 lines).
- `the_printed_hint_runs_in_a_shell_with_the_same_effect_as_the_one_token_form` and `the_runtime_hint_run_by_a_shell_matches_the_one_token_form` in cli/tests/set_split_reference.rs.

The gate the done-when commands ran (`cargo clippy --workspace --all-targets -- -D warnings`) does not pass the flag, so none of them failed a gate.

### Scope

- Each listed function is split to 30 lines or fewer with no behavior change, plus `rewrite_references` in living-docs-core/src/commands/set/retitle/references.rs, which the same flag reports, and any other function the flag reports once these are fixed (clippy stops at the first failing crate, so the cli crate's full list shows only after the core is clean).
- The workspace clippy gate the project runs enforces `too_many_lines` for every crate.

### Acceptance

- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` is clean.
- The full suite, `make check` and `se-gates check` pass with the same test count.

### Plan

1. Split each function, one commit.
2. Add the flag to the workspace clippy target.

### Outcome

Every function in the workspace is now 30 lines or fewer, and the workspace enforces it: the root Cargo.toml declares `[workspace.lints.clippy] too_many_lines = "warn"` and each member opts in with `[lints] workspace = true`, so `cargo clippy --workspace --all-targets -- -D warnings` fails on a longer function in any crate, tests included, the same as CI's `make cli-clippy`. Split with no behavior change: `supersede` (living-docs-core/src/commands/supersede/mod.rs, both drafts still built before any write), `append_unreached` (index/root.rs), `rewrite_references` (set/retitle/references.rs), `run_set` (cli/src/commands/set.rs) and three tests in cli/tests/set_read_back/crlf.rs and cli/tests/set_split_reference.rs; no `#[allow]` was added. Found blocking the ship of the usage-audit branch, where CI's clippy failed on the first three. Suite 1192/0 with the same test names before and after; review approved, its docblock finding fixed.
