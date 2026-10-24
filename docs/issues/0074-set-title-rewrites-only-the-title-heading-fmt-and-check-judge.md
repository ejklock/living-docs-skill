---
type: Issue
title: set title rewrites only the title heading fmt and check judge
description: set title renames a section heading when the record has no numbered title heading
status: open
timestamp: 2026-10-24T13:01:43Z
---

## 0074. set title rewrites only the title heading fmt and check judge

Found by the review of the fmt repair work (2026-10-24): `living-docs set <ref> title "T"` on a record whose body has no numbered title heading rewrites the first heading of any kind, so `## Context` becomes `## T`. `fmt` and `check` already judge the title heading by the record's own number prefix, falling back to a level-1 heading; `set title` still uses the first heading.

### Scope

- `set title` rewrites the same title heading `fmt` aligns and `check` judges, and leaves the body untouched when there is none.

### Acceptance

- A numbered record whose body starts with `## Context` and has no title heading: `set title "T"` writes the frontmatter title and leaves the body byte-identical.
- A record with `# 0003. Old` below a fenced `# 0003. Fake`: only the unfenced heading changes.
- The existing `set title` tests stay green.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: `set title` uses the shared title-heading lookup, tests red first.
