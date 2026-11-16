---
type: Issue
title: The changed-only pre-commit drops staged files whose names hold a space
description: Read staged names NUL-separated in changed-only mode so a path with a space is still checked
status: closed
timestamp: 2026-11-16T20:26:44Z
---

## 0146. The changed-only pre-commit drops staged files whose names hold a space

### Scope

In changed-only mode `skills/living-docs/hooks/pre-commit` expands the staged list unquoted, so a staged record named `docs/in dex.md` splits into two words and falls out of the checked scope; the commit passes unchecked. Found by the review of issue 0142; the behavior predates it.

### Decision

- Read the staged names NUL-separated (`git diff -z`) into an array and pass each as one argument.

### Acceptance

- A broken staged record whose path holds a space fails the changed-only commit; a test in `cli/tests/pre_commit_changed_only_spaces.rs` proves it; `.githooks/pre-commit` stays byte-identical to the distributed hook.

### Outcome

- The changed-only hook reads staged names NUL-separated into an array and passes each as `--changed-files=<path>`, so a path with a space, a quote or a leading dash (a bundle at the repository root) is checked as one argument; an empty list still exits 0 before the expansion (bash 3.2 safe). Proof: `cli/tests/pre_commit_changed_only_spaces.rs` (red against the previous hook).
- Tracked separately: `manage index` writes a link to a record whose name holds a space without angle brackets (issue opened 2026-11-16).
