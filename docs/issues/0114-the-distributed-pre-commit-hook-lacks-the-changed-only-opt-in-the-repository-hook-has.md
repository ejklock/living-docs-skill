---
type: Issue
title: The distributed pre-commit hook lacks the changed-only opt-in the repository hook has
description: "Make the distributed pre-commit hook the single source: it gains the changed-only opt-in and an owner opt-in, and the repository hook becomes its installed copy."
status: open
timestamp: 2026-11-12T17:16:18Z
---

## 0114. The distributed pre-commit hook lacks the changed-only opt-in the repository hook has

The distributed pre-commit hook `skills/living-docs/hooks/pre-commit` and this repository's `.githooks/pre-commit` are two hand-kept copies that have drifted: only the repository copy carries the `LIVING_DOCS_CHECK_CHANGED_ONLY` opt-in of [ADR 0062](/adr/0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md) and `--require-owner`, so a project that runs `living-docs manage install hooks` cannot arm the gate over a bundle with legacy debt. The duplicate-code gate ignores `.githooks/**` until the two become one source.

### Scope

- The distributed hook gains the changed-only opt-in and a way to require owners without editing the file.
- This repository's `.githooks/pre-commit` becomes exactly what `manage install hooks` writes, and the `.githooks/**` ignore in `.jscpd.json` narrows to `.githooks/pre-commit`.
- Kept: the binary order (repository build, then PATH) and the fail-open skip.

### Acceptance

- `LIVING_DOCS_CHECK_CHANGED_ONLY=1` narrows the verdict in a project that installed the hook with `manage install hooks`.
- `.githooks/pre-commit` is byte-identical to the installed copy, and a test fails when they differ.
- `se-gates check` passes with `.jscpd.json` ignoring only `.githooks/pre-commit` under `.githooks/`.

### Decision

- Implements [ADR 0081](/adr/0081-the-distributed-pre-commit-hook-is-the-only-copy-and-environment-variables-opt-it-into-changed-only-and-owner-checks.md).
- A project requires owners by setting `LIVING_DOCS_REQUIRE_OWNER=1`, the same opt-in shape as `LIVING_DOCS_CHECK_CHANGED_ONLY`. Because this repository's hook then stops forcing `--require-owner`, `make check` gains `check --require-owner docs`, so CI keeps requiring owners here. An ADR fixes the hook's contract before code. Not taken: a `living-docs.requireOwner` git config per clone; always requiring owners, which breaks projects that already installed the hook; dropping the owner requirement (owner, 2026-11-12).
- The test for `LIVING_DOCS_REQUIRE_OWNER=1` over the whole bundle runs on a repository whose only defect is the ownerless ADR, so it fails when the hook drops `--require-owner`. The first review found the case built on a repository with legacy debt, which failed the bundle whether or not owners were required. Not taken: shipping with the weak test and strengthening it in a separate issue (owner, 2026-11-12).
- `.jscpd.json` keeps ignoring `.githooks/pre-commit` alone: the duplicate-code gate reports any byte-identical copy, a symlink included (it follows the link), so the repository copy cannot leave its ignore; `pre_commit_single_source` guards the drift instead. Not taken: removing `.githooks/pre-commit` from git and having each clone run `living-docs manage install hooks` (owner, 2026-11-12).
