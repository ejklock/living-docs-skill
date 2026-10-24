---
type: Issue
title: A record with no title gets one without its number and never from fenced text
description: extract_record derives a missing title from the filename stem with its NNNN prefix or from a fenced heading
status: open
timestamp: 2026-10-24T13:01:42Z
---

## 0073. A record with no title gets one without its number and never from fenced text

Found by the review of the fmt repair work (2026-10-24): a record with no frontmatter `title` and no level-1 heading gets a title derived from its filename stem by `extract_record`, number included (`0012-k.md` yields a title carrying `0012`), and a record whose only numbered heading sits inside a fenced block gets that fenced text as its title.

### Scope

- `extract_record` derives a missing title without the `NNNN` prefix and never from a heading inside a fenced code block.

### Acceptance

- `0012-some-thing.md` with no title and no heading: the derived title carries no `0012`.
- A record whose only `# 0012. X` heading is fenced: no title is taken from it.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: title derivation in the record extractor, tests red first.
