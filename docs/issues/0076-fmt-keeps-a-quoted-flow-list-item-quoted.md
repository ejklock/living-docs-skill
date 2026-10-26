---
type: Issue
title: fmt keeps a quoted flow-list item quoted
description: fmt writes a list item quoted when its plain form would split it or change its type, so a tag holding a comma or a bracket survives.
status: closed
timestamp: 2026-10-24T22:46:53Z
---

## 0076. fmt keeps a quoted flow-list item quoted

### Scope

- Every flow sequence the canonical pass writes (`tags` and every tail sequence) quotes an item whose plain form would read back as anything other than the same single string: an item holding a flow indicator (`,` `[` `]` `{` `}`), or one whose plain form reads as null, a boolean or a number.
- A record whose list items need no quoting keeps the bytes it has today.

### Acceptance

- `tags: ["a, b", "x]"]` survives `fmt` with its two tags, and a second `fmt` changes nothing.
- `tags: ["~", "null", "true"]` keeps three string tags through `fmt`.
- `tags: [a, b]` stays byte-identical through `fmt`.
- `check` raises no non-canonical finding for a list `fmt` just wrote.

### Plan

A flow-list item goes through a flow-context quoting rule instead of the block-scalar one. Reproduced on 8d797e8: an ADR with `tags: ["a, b", "x]"]`, after `living-docs fmt <bundle> --plain`, reads `tags: [a, b, x]]`: three tags in place of two, and a stray bracket.

### Outcome

Delivered 2026-10-25 on branch task/ek-usage-audit-backlog, together with part 3 of issue 0066 (supersede keeps an earlier `supersedes` value), because both changed the canonical writer.

- Every flow sequence the canonical pass writes (`tags`, `supersedes` and each tail sequence) quotes an item by the shared rule plus a flow-context read-back: an item with a flow indicator, a line break, or a plain form that reads as null, a boolean or a number is written quoted. A list whose items need no quoting keeps its bytes.
- Owner decision: a scalar is quoted exactly when its plain `key: value` form would read back as another string (`1.50`, `~`, `null`, `0x10`, `.5`, `1e3`, `+1`, a leading space); `.inf`, `true`, `3`, `0001`, dates and `a, b` stay plain because they read back unchanged as text.
- Owner decision: the separate line-break trigger in the quoting rule was deleted as dead code; the read-back is the one defense, and a planted mutant without it turns three `supersede_canonical` tests red.
- `fmt`, `supersede`, the callout pass and `check`'s non-canonical finding share one read-back guard: a record whose canonical form would change a value is refused instead of being rewritten.
- `supersedes` is a typed list (`Option<Vec<String>>`); `set number` refuses a record named by any list entry, not only by a scalar link.
- Tests: `cli/tests/fmt_flow_list.rs`, `cli/tests/supersede_canonical.rs`, `cli/tests/supersede_keeps.rs`, `cli/tests/set_tracker_keys.rs`, `cli/tests/set_number.rs`, and unit tests for the writer, the extractor and the lineage.
- Review: a full review, two narrow re-reviews and an integration review; every blocking and normal finding was fixed. Defects found along the way and older than this work are tracked as issues 0084 to 0088 and 0090.
- Verification in the main checkout: `cargo test --workspace` 1113 passed, 0 failed; clippy with `-D warnings`, `cargo fmt --check`, `make check` (file-size ratchet with the `record.rs` baseline line dropped, allow-inventory), `living-docs check docs`, `fmt docs --check` and `se-gates check` pass.
