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
- This repository's `.githooks/pre-commit` becomes exactly what `manage install hooks` writes, and the `.githooks/**` ignore leaves `.jscpd.json`.
- Kept: the binary order (repository build, then PATH) and the fail-open skip.

### Acceptance

- `LIVING_DOCS_CHECK_CHANGED_ONLY=1` narrows the verdict in a project that installed the hook with `manage install hooks`.
- `.githooks/pre-commit` is byte-identical to the installed copy, and a test fails when they differ.
- `se-gates check` passes with no `.githooks/**` ignore in `.jscpd.json`.
