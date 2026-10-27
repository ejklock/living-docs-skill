---
type: Issue
title: integration tests leave their scratch directories behind
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-27T00:47:08Z
---

## 0100. integration tests leave their scratch directories behind

### Scope

One run of `cargo test --workspace` leaves 446 directories in `$TMPDIR`, all created by `cli/tests`. Of those, 444 are named `living-docs-<suite>-test-<label>-<nanos>`, across 41 suites, and 2 carry only a nanosecond number. The shared helper `common::temp_bundle` and the per-file `temp_dir`/`temp_bundle` helpers create a directory and never remove it, so the leftovers pile up into thousands across runs. The unit tests in `living-docs-core` and `fs-store` already remove theirs.

### Decision

- Every scratch directory a `cli/tests` helper creates is owned by a guard that removes it when dropped. The guard extends the `Scratch` pattern `cli/tests/check_hook_hint.rs` already uses. No new dependency: `tempfile` was rejected in favour of the project's own pattern.
- The guard lives in `cli/tests/common/mod.rs`. The per-file helpers return it instead of a bare `PathBuf`.
- Test behavior and assertions do not change; only the lifetime of the scratch directory does.

### Acceptance

- After `cargo test --workspace` runs with `TMPDIR` set to an empty directory, that directory is empty.
- The whole suite passes with the same test count as before.
- A test that panics still removes its directory, because the guard drops during unwinding.

### Outcome

Every scratch directory a `cli/tests` test creates is now owned by `Scratch` in `cli/tests/common/mod.rs`, a guard that removes its root on drop, including when a test panics. `common::temp_bundle` and the per-file helpers return it, and the copy in `check_hook_hint.rs` is folded into the common one.

- **Naming:** the root is named `<name>-<pid>-<nanos>-<serial>`. `Scratch::new` derefs to the root itself, and `with_subdir` derefs to a directory inside it.
- **Early drops:** several tests built a guard and kept only a path derived from it, which removed the directory under the test. Each one now keeps its guard alive until its last use.
- **Fixed-name files:** the two fixed-name scratch files in `set_read_back` now live inside a guarded directory.
- **Odd entries:** the two `-<nanos>` entries came from a multi-line label in `set_number.rs`. Both were halves of one directory name that contained a newline.

Verified:

- With `TMPDIR` set to an empty directory, `cargo test --workspace` passes with 1278 tests and leaves 0 entries; before, it left 446.
- `cli/tests/common_scratch.rs` proves the guard's contract:
  - the directory exists while the guard lives;
  - the root and its children are gone after a drop, and after a panic;
  - `Scratch::new` has no trailing separator;
  - 800 guards created from 16 threads released by one `Barrier` never share a path. With the serial dropped from the name, this test fails in every run.

Left as is: the manual `remove_dir_all` calls that some tests still make are now redundant, but harmless.
