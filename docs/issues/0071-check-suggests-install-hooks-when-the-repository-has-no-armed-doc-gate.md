---
type: Issue
title: check suggests install hooks when the repository has no armed doc-gate
description: Bundles with the pre-commit doc-gate armed all pass and most without it fail; check prints a one-line stderr hint naming living-docs install hooks when it finds no armed gate, without changing its output or exit code
status: open
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
