---
type: Issue
title: fmt keeps a CRLF record's single frontmatter block and line endings
description: A CRLF record gains a second frontmatter block from fmt; fmt and check must read CRLF frontmatter as the record's own
status: closed
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

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog, folded into issue 0059's work by owner decision; [ADR 0072](/adr/0072-records-keep-their-own-line-endings-crlf-is-read-and-written-as-crlf-and-a-mixed-ending-record-fails-check.md) records the design.

- A store view in core (`living-docs-core/src/line_ending.rs`) reads a record whose every line break is CRLF as LF and writes it back as CRLF; `set`, `supersede`, `fmt` and `check` go through it, and the type-index refresh converts at its own edge. No parser changed.
- `fmt` on a CRLF ADR with unordered keys leaves one frontmatter block, keys in canonical order and CRLF endings; a second run writes nothing. `check` on a CRLF record reports what it reports on its LF twin.
- Scope grew to `set` and `supersede` (owner decision): each write on a CRLF record equals the LF twin's result with CRLF, byte for byte.
- A record with mixed endings is refused by `set`, `supersede` and `fmt` before any write and fails `check` with one violation naming it. Siblings, prose and reserved index or log files are read and rewritten as before.
- Tests and verification: see issue 0059's Outcome (947 passed, 0 failed; all gates green).
