---
type: Issue
title: check suggests install hooks when the repository has no armed doc-gate
description: Bundles with the pre-commit doc-gate armed all pass and most without it fail; check prints a one-line stderr hint naming living-docs install hooks when it finds no armed gate, without changing its output or exit code
status: closed
timestamp: 2026-10-24T10:17:34Z
---

## 0071. check suggests install hooks when the repository has no armed doc-gate

Found by the usage audit: of 41 bundles under the owner's workspace, the six whose repository arms the pre-commit doc-gate all pass `check`, and 25 of the others fail. [ADR 0021](/adr/0021-enforcement-layers-ship-with-the-repo-write-gate-hook-session-teaching-and-pre-commit-doc-gate.md) puts enforcement at commit, and `living-docs install hooks` is the verb that arms it.

### Scope

- When `check` runs inside a git repository whose pre-commit hook (through `core.hooksPath` or `.git/hooks`) does not run `living-docs check`, it prints one informational line on stderr naming `living-docs install hooks`.
- The line is silenced by `--quiet`; stdout, the JSON document and the exit code are unchanged.

### Decision

- A stderr hint, not an advisory: an advisory belongs to a record, and the hint is about the repository.

### Acceptance

- In a git repository without the hook, `check` prints the hint on stderr; with `install hooks` run first, it does not; outside a git repository, it does not.
- `--quiet` removes the hint; the stdout of `check --json` and the exit code are identical with and without it.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. CLI: the armed-gate detection and the hint; tests through the binary.

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog.

- `cli/src/doc_gate.rs` (new): `doc_gate_armed(dir)` asks `git -C <dir> rev-parse --git-path hooks` for the hooks directory, joins a relative answer onto `dir`, and reports the gate armed when its `pre-commit` text names both `living-docs` and `check`; outside a git repository it answers nothing and no hint prints. The flag `--path-format=absolute` is not used, so git older than 2.31 works.
- `cli/src/commands/check.rs`: `note_missing_doc_gate` prints one stderr line suggesting `living-docs install hooks` through the informational channel `--quiet` silences; stdout, the JSON document and the exit code are unchanged. `run_check` stays under the 30-line limit.
- `cli/src/hooks.rs` shares the git helper and shrank to 521 lines; its file-size baseline was tightened to match.
- `cli/tests/check_hook_hint.rs`: 11 tests (no hook, installed hooks, a `.git/hooks` gate, a foreign pre-commit, a non-repository, a nested bundle, a relative `core.hooksPath`, `--quiet`, JSON byte-identity, equal exit codes), isolated from HOME and the global git config. A planted cwd-relative join failed 5 of them.
- Verification in the main checkout: `cargo test --workspace` 803 passed, 0 failed; `make check`, the file-size ratchet, clippy with `-D warnings`, `cargo fmt --check` and `se-gates check` pass. Review approved; narrow re-review of the fixes approved with one nit: the relative-hooksPath test runs with cwd equal to the bundle, so only the nested-bundle test guards the join.
