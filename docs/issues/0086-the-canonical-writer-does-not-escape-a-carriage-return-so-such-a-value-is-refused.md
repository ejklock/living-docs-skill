---
type: Issue
title: the canonical writer does not escape a carriage return so such a value is refused
description: <One sentence — the change and its motivation.>
status: closed
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

### Outcome

The canonical writer now escapes a carriage return inside a double-quoted value as the two characters `\r` (`quoted` in living-docs-core/src/record/canonical.rs, after the backslash escape), so a value holding one is written, reads back unchanged and is no longer refused by the read-back guard. This covers a description, a tail scalar and flow-list items; a CRLF record keeps its line endings and the escape stays two characters; `set <ref> description --from` a file holding a carriage return writes `description: "a\rb"`. A tab is not refused today, so tabs are left as they are. Tests: cli/tests/fmt_carriage_return.rs (6) and a round-trip unit test in living-docs-core/src/record/tests/canonical.rs; each fails when the escape is reverted. Suite 1192/0, review approved with two nits: a long docblock line in `format_scalar`, and no dedicated test for a carriage return at the start, the end or alone (probed fine).
