---
type: Issue
title: the file-size and allow-inventory gates fail inside a git worktree under .claude/worktrees
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:55:52Z
---

## 0091. the file-size and allow-inventory gates fail inside a git worktree under .claude/worktrees

Found while running coders in git worktrees under `.claude/worktrees/` (2026-10-25): `scripts/check-file-size.sh` and `scripts/check-allow-inventory.sh` skip every path matching `*/.claude/worktrees/*`. That keeps the main checkout from scanning nested worktrees, but run from inside a worktree the pattern matches the worktree's own files, so the scripts see no `.rs` file, report every baseline entry as "no longer exists", and `make check` exits 2 on an unchanged tree.

### Scope

- Both scripts skip only worktrees nested below the directory they check, judged relative to that root, so `make check` passes inside a clean worktree exactly as it does in the main checkout.

### Acceptance

- In a fresh `git worktree add .claude/worktrees/x HEAD`, `make check` exits 0.
- In the main checkout with a nested worktree present, the scripts still ignore the nested worktree's files.
- A file over 300 lines added inside the worktree still fails the ratchet there.

### Plan

1. Scripts: the exclusion relative to the checked root, proved by the three cases.
