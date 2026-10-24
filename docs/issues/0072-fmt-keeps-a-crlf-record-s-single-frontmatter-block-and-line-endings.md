---
type: Issue
title: fmt keeps a CRLF record's single frontmatter block and line endings
description: A CRLF record gains a second frontmatter block from fmt; fmt and check must read CRLF frontmatter as the record's own
status: open
timestamp: 2026-10-24T13:01:41Z
---

## 0072. fmt keeps a CRLF record's single frontmatter block and line endings

Found by the review of the fmt repair work (2026-10-24, reproduced in a temporary bundle with the v0.23.0 binary): a record whose lines end in CRLF gets a second frontmatter block from `living-docs fmt`, because the frontmatter is detected by a bare `---` line and the CRLF opening line is `---\r`.

### Scope

- `fmt` and the canonical serializer recognise a CRLF frontmatter block as the record's frontmatter and rewrite it in place, keeping the record's line endings.
- `check` judges a CRLF record the same way it judges the LF one.

### Acceptance

- A CRLF ADR with unordered frontmatter keys: `fmt` leaves one frontmatter block, keys in canonical order, every line still ending in CRLF; a second `fmt` writes nothing.
- `check` on the CRLF record reports the same violations as on its LF twin.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: frontmatter detection tolerant of CRLF, tests red first.
