---
type: Issue
title: an index row splits across two lines when a record title holds a line break
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:06:18Z
---

## 0090. an index row splits across two lines when a record title holds a line break

Found by the re-review of the canonical writer (2026-10-24): when a record's title holds a line break (`title: "one\ntwo"`, which the canonical writer now keeps quoted), `index` writes the record's bullet across two lines, `* [0001 — one` and `two](0001-one-two.md)`. The link breaks in the rendered index, and the row no longer reads as one entry.

### Scope

- An index row always fits on one line, whatever the title holds.

### Acceptance

- A record titled `"one\ntwo"` gets a single-line bullet in its type index, linking to its file; `index` run twice is byte-identical; `check` reports no index drift.
- A title without a line break keeps today's bullet byte for byte.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the index row renderer, tests red first.
