---
type: Issue
title: fmt rewrites a record whose frontmatter does not parse and drops its keys
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:00:35Z
---

## 0087. fmt rewrites a record whose frontmatter does not parse and drops its keys

Found by the review of the canonical supersedes work (2026-10-24): when a record's frontmatter does not parse as YAML (for example `aliases: [a` with no closing bracket), `fmt` rewrites the record with `type: ""` and drops owner, status and every other key. The author loses the frontmatter instead of being told it is broken. The same happens on the base.

### Scope

- `fmt` and `set` refuse a record whose frontmatter does not parse, without writing it, naming the record; `check` reports it as unparseable instead of as a record missing its keys.

### Acceptance

- A record with `aliases: [a`: `fmt` exits 2 naming it, the file is byte-identical, and every other pending record is still written.
- `set` on that record exits 2 with the file unchanged.
- `check` reports one violation saying the frontmatter does not parse.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the frontmatter read in the canonical path, tests red first.
