---
type: Issue
title: The file-size ratchet reports a stale baseline inside a fleet worktree
description: Exclude only nested worktrees from the file-size ratchet, so make check gives the same verdict in a worktree under .claude/worktrees as in the main checkout.
status: closed
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

### Decision

- Both scripts enumerate with `find .` run from `$root` and prune `./.claude/worktrees`, `./target` and `./.git` by relative path. The reviewer of the first fix found that an absolute `"$root/.claude/worktrees/*"` pattern stops matching when the root path holds a `[` (find reads `-path` as a glob), and, as pre-existing defects in the same expression, that the unanchored `target/` and `.git/` patterns drop every file of a repository under any `/target/` directory, and that the scan walks every sibling worktree (5.17 s against 0.02 s with `-prune`). One relative, pruned expression fixes all three, so they ship here. Not taken: escaping glob characters in `$root` and tracking `target/`, `.git/` and the speed in a separate issue, which would rewrite the same line twice (session, 2026-11-12).
