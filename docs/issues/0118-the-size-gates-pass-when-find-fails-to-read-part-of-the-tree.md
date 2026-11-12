---
type: Issue
title: The size gates pass when find fails to read part of the tree
description: Fail the file-size and allow-inventory gates when find cannot read part of the tree, instead of judging a partial file list.
status: open
timestamp: 2026-11-12T21:08:04Z
---

## 0118. The size gates pass when find fails to read part of the tree

`scripts/check-file-size.sh` and `scripts/check-allow-inventory.sh` read the `.rs` list from `find` through a process substitution (`done < <(cd "$root" && find ...)`, in `enumerate_and_check`). Bash ignores the exit status of a process substitution even under `set -euo pipefail`, so a `find` error never fails the gate: the files `find` could not reach are skipped and the gate prints OK.

Reproduction (reviewer of issue 0116, 2026-11-12): a synthetic repository with a 301-line `src/locked/y.rs` and `chmod 000 src/locked` prints "find: ./src/locked: Permission denied", then "File-size ratchet OK", exit 0. The base script at 4010c5e behaves the same, so issue 0116 did not introduce it.

### Scope

- Both scripts fail (exit 1, with a message naming the enumeration) when `find` exits non-zero, instead of judging a partial list.
- Kept: the ratchet rules, the baseline format, the relative paths and messages the scripts print, and the pruned enumeration from issue 0116.

### Acceptance

- In each harness under `scripts/tests/`, a synthetic repository with an unreadable directory makes the script exit 1; every existing case stays green.
- `make check` in the main checkout still passes.
