---
type: Issue
title: fmt drops a tail key whose value is the empty string
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-10-25T02:00:28Z
---

## 0084. fmt drops a tail key whose value is the empty string

Found by the review of the canonical supersedes work (2026-10-24): a frontmatter key outside the typed columns whose value is an empty string (`alias: ""`) disappears when `fmt` rewrites the record. The value reads back as empty text, so the canonical form loses a key the author wrote. The same happens on the base before that work.

### Scope

- The canonical writer keeps a tail key whose value is the empty string, written as `key: ""`.

### Acceptance

- An ADR with `alias: ""` keeps the line `alias: ""` through `fmt`, a second `fmt` changes nothing, and `check` is clean.
- A tail key with no value at all (`alias:`) keeps today's behavior, named in the test.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the tail scalar path of the canonical writer, tests red first.

### Outcome

`fmt` now keeps a tail key whose value is the empty string, written as `key: ""` (a single-quoted `''` is written double-quoted), and an empty item in a tail sequence stays an empty item; a CRLF record keeps its line endings. The tail extraction moved to living-docs-core/src/record/tail.rs. A tail key with no value at all (`alias:`) is still dropped, as before, named in cli/tests/fmt_empty_tail.rs. A blank `timestamp` (empty, whitespace only, null, `~` or a mapping) is still reseeded by `fmt` (living-docs-core/src/commands/fmt/repair.rs, seed_timestamp). The read-back guard (living-docs-core/src/commands/supersede/guard.rs, unwritable_key) now compares an empty-string tail value, so a writer that dropped or changed `alias: ""` is refused instead of losing the key silently; a scalar `supersedes: ""`, which the canonical pass drops, stays uncompared. Tests: cli/tests/fmt_empty_tail.rs and the guard unit tests; suite 1176/0, reviews approved (one nit: the blank typed-key case is tested with `description`, which shares the `title` path).
