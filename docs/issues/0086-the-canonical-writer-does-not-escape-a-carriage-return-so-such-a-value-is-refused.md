---
type: Issue
title: the canonical writer does not escape a carriage return so such a value is refused
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:00:33Z
---

## 0086. the canonical writer does not escape a carriage return so such a value is refused

Found by the review of the canonical supersedes work (2026-10-24): the canonical writer's double-quoted form escapes a backslash, a quote and a line feed but not a carriage return. A value holding `\r` (`description: "a\rb"`) therefore cannot be written canonically: the read-back guard refuses the record and `check` asks for a hand edit. Before the guard, the value was silently changed.

### Scope

- The double-quoted form escapes a carriage return as `\r`, so such a value is written and reads back unchanged.

### Acceptance

- An ADR with `description: "a\rb"` goes through `fmt` with exit 0, keeps `description: "a\rb"`, a second `fmt` changes nothing, and `check` is clean.
- `set adr/NNNN description` with a value holding a carriage return writes the same escaped line.
- A record with CRLF line endings keeps its line endings.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the quoting in the canonical writer, tests red first.
