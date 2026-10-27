---
type: Issue
title: integration tests keep redundant manual cleanup calls
description: Drop the manual test cleanup calls that the Scratch guard already makes, keeping the ones that set up a case.
status: closed
timestamp: 2026-10-27T02:38:50Z
---

## 0103. integration tests keep redundant manual cleanup calls

### Scope

Since issue 0100, a `Scratch` guard removes every integration-test scratch directory when the test ends. Many tests in `cli/tests` still call `fs::remove_dir_all` themselves at the end of a test, on a path the guard already owns. Those calls are now noise.

Not every `remove_dir_all` call is redundant. Some tests delete a directory or a file mid-test to set up the case they prove, for example a missing bundle root or a removed source file. Those calls are part of the test.

### Decision

- Remove each `remove_dir_all` (or `remove_file`) call that only cleans up a path a live `Scratch` guard owns. Keep every call that sets up the case a test proves.
- Change no assertion and add no test. The suite count stays the same.

### Acceptance

- Every remaining `remove_dir_all` call in `cli/tests`, outside `common/mod.rs`, either sets up the case its test proves or cleans a path no guard owns. The PR lists the kept calls.
- With `TMPDIR` set to an empty directory, `cargo test --workspace` passes with the same test count and leaves 0 entries.
- The file-size baseline only shrinks.

### Outcome

126 redundant cleanup calls are gone from 20 files in `cli/tests`, 252 lines in total. Each one ended a test or a helper by removing a path that a live `Scratch` guard already owns. The two `cleanup` helpers that only wrapped that call are deleted, and so is the trailing removal inside `assert_clean_no_broken_link` and `assert_clean`.

The file-size baseline shrinks: `check_core.rs` goes from 336 to 317 lines, `hooks_install.rs` from 661 to 623 and `index_supersede.rs` from 725 to 685.

Each removal that stays sets up the case its test proves, or cleans a path no guard owns:

- `gitattributes/scope.rs:75` cleans a sibling directory outside the guard's root.
- `gitattributes/scope.rs:152` deletes `docs` mid-test, before the uninstall that depends on its absence.
- The `remove_file` calls in `fresh_bundle.rs`, `output_contract_authoring.rs`, `reference_paths.rs` and `rooted_store.rs` delete an index, a source file or a symlink mid-test.

Verified: with `TMPDIR` set to an empty directory, `cargo test --workspace` passes with the same 1278 tests and leaves 0 entries. No test or assertion changed.
