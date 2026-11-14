---
type: Issue
title: Two parity test helpers exceed the function line cap and fail clippy
description: Shorten parse_row and make_bundle under 30 lines so clippy -D warnings passes, as CI's make cli-clippy requires.
status: open
timestamp: 2026-11-13T01:21:16Z
---

## 0120. Two parity test helpers exceed the function line cap and fail clippy

Two test helpers exceed the 30-line function cap that `clippy::too_many_lines` enforces with `-D warnings`: `parse_row` in `cli/tests/common/parity.rs` (31 lines, from a7fb1cb) and `make_bundle` in `cli/tests/common/parity_bundles.rs` (32 lines, from b680316). `cargo clippy --manifest-path Cargo.toml --workspace --all-targets -- -D warnings` fails on both, so CI's `make cli-clippy` fails this branch. Both commits passed `se-gates check` because its `clippy-lint` check is non-blocking (`"blocking":false`, reported as a pass with omitted findings).

### Scope

- Bring `parse_row` and `make_bundle` to 30 lines or fewer by extracting a well-named step, with no change to the parity cases they build.
- Kept: every parity test and its fixtures.

### Acceptance

- `cargo clippy --workspace --all-targets -- -D warnings -W clippy::too_many_lines` exits 0.
- `cargo test --workspace` passes with the same count.
