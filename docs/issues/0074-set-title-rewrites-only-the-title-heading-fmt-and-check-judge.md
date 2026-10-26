---
type: Issue
title: set title rewrites only the title heading fmt and check judge
description: set title renames a section heading when the record has no numbered title heading
status: closed
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

### Outcome

`set title` now rewrites only the record's own title heading, found by `replace_title_heading` (living-docs-core/src/commands/set/retitle.rs): the heading numbered like the file, else an unnumbered level-1 heading. A section heading is never renamed, and a heading without a number prefix keeps having none. Tests: living-docs-core/src/commands/set/retitle/tests.rs and fmt/heading_tests.rs; suite 1119/0 at integration, review approved. A possible edge case of the unnumbered fallback is tracked in issue 0093.
