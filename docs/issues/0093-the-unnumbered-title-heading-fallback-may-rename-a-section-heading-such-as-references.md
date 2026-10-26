---
type: Issue
title: the unnumbered title-heading fallback may rename a section heading such as References
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T03:10:31Z
---

## 0093. the unnumbered title-heading fallback may rename a section heading such as References

`set title` and `fmt` find a record's title heading with `title_heading_index` (living-docs-core/src/commands/set/retitle.rs): the heading numbered like the file, else the first unnumbered level-1 heading. On a record whose title heading lacks its number, while its body holds another level-1 section heading such as `# References`, the fallback can pick that section heading and rename it. This is an inference from reading the code during the review of the set-title slice; it has not been reproduced yet.

### Scope

- living-docs-core/src/commands/set/retitle.rs › title_heading_index and its callers in `set title` and `fmt`.

### Acceptance

- Reproduce first: a record with no numbered title heading and a later `# References` heading, run `set <TYPE/NNNN> title X` and `fmt`. If neither renames `# References`, close the issue as not reproduced, with the commands that proved it.
- If it reproduces: neither verb renames a heading that is not the record's title heading; a test names the case.

### Plan

Restrict the unnumbered fallback to the first level-1 heading of the body, before any section, or to a heading that matches the current title.
