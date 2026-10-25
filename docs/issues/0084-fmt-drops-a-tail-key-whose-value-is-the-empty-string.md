---
type: Issue
title: fmt drops a tail key whose value is the empty string
description: <One sentence — the change and its motivation.>
status: open
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
