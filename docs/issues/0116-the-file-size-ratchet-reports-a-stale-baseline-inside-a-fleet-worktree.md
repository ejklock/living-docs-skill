---
type: Issue
title: The file-size ratchet reports a stale baseline inside a fleet worktree
description: Exclude only nested worktrees from the file-size ratchet, so make check gives the same verdict in a worktree under .claude/worktrees as in the main checkout.
status: open
timestamp: 2026-11-12T18:19:45Z
---

## 0116. The file-size ratchet reports a stale baseline inside a fleet worktree

`scripts/check-file-size.sh` enumerates `*.rs` files with `find "$root" ... -not -path '*/.claude/worktrees/*'` (near line 88). The pattern matches the absolute path, so when the checkout itself lives under `.claude/worktrees/` (a fleet worktree), every file is excluded, every baseline entry reads as stale, and `make check` fails with "STALE BASELINE" in a worktree whose tree is clean. A coder in a fleet worktree cannot run the full `make check` its brief asks for.

### Scope

- The exclusion applies to worktrees nested inside the checkout being checked, never to the checkout's own location: match on the path relative to `$root`.
- The same fix applies to `scripts/check-allow-inventory.sh`, which enumerates with the same `-not -path '*/.claude/worktrees/*'` and reports the same stale entries inside a worktree (seen by the issue 0112 coder, 2026-11-12).
- Kept: the ratchet rules, the baseline format and the exclusion of nested worktrees from the main checkout.

### Acceptance

- `make check` (the file-size and allow-inventory steps) gives the same verdict in a worktree under `.claude/worktrees/` as in the main checkout at the same commit.
- A `.rs` file inside a nested `.claude/worktrees/<name>/` of the main checkout is still not counted by either script; the fixtures under `scripts/tests/check-file-size/` and `scripts/tests/check-allow-inventory/` cover both cases.
