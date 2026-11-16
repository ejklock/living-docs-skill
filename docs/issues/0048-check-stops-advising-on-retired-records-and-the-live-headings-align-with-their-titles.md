---
type: Issue
title: check stops advising on retired records and the live headings align with their titles
description: check stops advising on records the CLI refuses to change, and the live headings align with their titles, so every remaining advisory is actionable
status: closed
timestamp: 2026-09-22T22:11:31Z
---

## 0048. check stops advising on retired records and the live headings align with their titles

`check` printed 54 advisories over this bundle; 39 sat on retired records that `set title` refuses to change, so they could never be cleared. Implements [ADR 0063](../adr/0063-authoring-advisories-skip-retired-records.md): the `HEADING` and `SIZE` advisories skip retired records. The same issue clears the 12 `HEADING` advisories left on live records, so the bundle's own `check` shows only findings that still matter.

### Scope

- One retired-record predicate in `core`: `status: Superseded` for any type, or a status in the type's `terminal_statuses`. `set title`'s refusal and both advisories use it.
- `HEADING` and `SIZE` skip a record the predicate calls retired.
- The 12 live records whose heading disagrees with their title get the heading rewritten to the title: nine ADRs (0017, 0028, 0029, 0047, 0048, 0057, 0058, 0059, 0060), research 0001 and 0002, and `constitution.md`.
- Kept: every invariant violation and `MOVED-SOURCE` still run over retired records. The two `SIZE` advisories on live ADRs 0017 and 0029 stay; shortening a decision record is an authoring task of its own.

### Decision

For each live record, the frontmatter `title` wins over the heading. The title is what the index, the filename slug and every link carry; rewriting the heading changes one line and renames nothing. The option not taken was to shorten each title to its heading, which would rename files and rewrite links across the bundle.

### Acceptance

- A retired record (`Deprecated`, `closed`, `done` or `Superseded`) with a heading that disagrees with its title produces no `HEADING` advisory; the same record while live produces one.
- A retired record over the size target produces no `SIZE` advisory; the same record while live produces one.
- A retired record with an invariant violation still fails `check`.
- `set title` on a retired record is still refused.
- `living-docs check` over `docs/` reports no `HEADING` advisory.

### Plan

1. The shared predicate, the two advisories and their tests (`core`).
2. The 12 live headings aligned to their titles (`docs/`).
