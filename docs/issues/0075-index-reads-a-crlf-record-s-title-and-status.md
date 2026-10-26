---
type: Issue
title: index reads a CRLF record's title and status
description: index reads every record through the shared line-ending view, so a CRLF record's title and status reach its index row.
status: closed
timestamp: 2026-10-24T20:33:26Z
---

## 0075. index reads a CRLF record's title and status

### Scope

- `index` reads each record through the same line-ending view as `check`, `set`, `supersede` and `fmt`, so a CRLF record's title and status reach its index row.
- The index-drift check keeps comparing against the same generation `index` writes, so it agrees with `index` before and after this change.

### Acceptance

- A bundle whose records are CRLF gets index rows with the same title and status as the LF form of the same records, and `index` prints no "no parseable 'title' frontmatter" warning for them.
- `index` output for an all-LF bundle stays byte-identical.
- After `index` on a CRLF bundle, `check` reports no index drift.
- A record with mixed line endings keeps its row, read as its LF form, and `index` warns on stderr naming the record; `check` reports it, and `set` and `supersede` refuse it.

### Plan

Wrap the store `index` reads records from in the line-ending view, after the union-merge slice frees index.rs. Reproduced on e7ef6c8: create an ADR, convert it to CRLF, run `living-docs index`, and the row reads `* [0001 — Alpha](0001-alpha.md) - ` with an empty status.

### Outcome

Delivered in 4b3152c. `index` reads every record through the shared line-ending view, so a CRLF record's title and status reach its index row, the architecture view included; a record with mixed line endings keeps its row and `index` alone prints a warning naming the file to stderr (living-docs-core/src/commands/index.rs; cli/tests/index_crlf.rs). Verified 2026-10-26: index_crlf 9/0.
