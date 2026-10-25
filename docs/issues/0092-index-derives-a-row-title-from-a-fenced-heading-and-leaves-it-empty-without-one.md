---
type: Issue
title: index derives a row title from a fenced heading and leaves it empty without one
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T03:10:30Z
---

## 0092. index derives a row title from a fenced heading and leaves it empty without one

`index` derives a record's row title with its own extractor (living-docs-core/src/commands/index.rs › title_for_record and first_heading), separate from the one `read` and `fmt` use. That extractor takes the first line starting with `# ` even inside a fenced code block, and leaves the title empty when the record has no level-1 heading, where the record extractor now falls back to the filename stem without its number. A record without a `title:` therefore gets a different title in its index row than in `read`. Found by the review of the derived-title slice, which compared both binaries on a bundle with a fenced-only heading: the row read `[0002 — Fake]` and `[0001 — ]`.

### Scope

- living-docs-core/src/commands/index.rs › title_for_record, first_heading.
- The record extractor in living-docs-core/src/record.rs is the reference behavior.

### Acceptance

- A record without `title:` whose only level-1 heading sits inside a fenced block gets the same row title from `index` as `read` shows.
- A record without `title:` and without a level-1 heading gets the filename stem without its number, as `read` shows.
- `index` stays idempotent and the existing index tests stay green.

### Plan

Make `index` use the record extractor's title, or share one fence-aware heading reader, so the two cannot drift again.
