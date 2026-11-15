---
type: Issue
title: write set reads a value that starts with a dash as a flag
description: A blocked_by or other value whose first character is - is rejected by the argument parser unless it follows --
status: open
timestamp: 2026-11-15T23:06:50Z
---

## 0137. write set reads a value that starts with a dash as a flag

### Scope

`living-docs write set <issue> blocked_by "-0, 12"` exits 2 with clap's "unexpected argument '-0' found": a value whose first character is `-` is read as a flag unless it comes after `--` (`write set --plain -- 0001 blocked_by "-0, 12"` works). The same holds for `-3` or `- x` first in the list. Found by the review of issue 0133 slice S1; it predates it.

### Decision

- Accept hyphen-leading values for the VALUE argument of `write set` (clap `allow_hyphen_values`), or document `--` in the issue-workflow rule; prefer the former so an agent does not need to know the parser rule.

### Acceptance

- `write set <issue> blocked_by "-3, 12"` succeeds without `--`, and a test proves it; flags after the value still parse.
