---
type: Issue
title: fmt keeps a quoted flow-list item quoted
description: fmt writes a list item quoted when its plain form would split it or change its type, so a tag holding a comma or a bracket survives.
status: open
timestamp: 2026-10-24T22:46:53Z
---

## 0076. fmt keeps a quoted flow-list item quoted

### Scope

- Every flow sequence the canonical pass writes (`tags` and every tail sequence) quotes an item whose plain form would read back as anything other than the same single string: an item holding a flow indicator (`,` `[` `]` `{` `}`), or one whose plain form reads as null, a boolean or a number.
- A record whose list items need no quoting keeps the bytes it has today.

### Acceptance

- `tags: ["a, b", "x]"]` survives `fmt` with its two tags, and a second `fmt` changes nothing.
- `tags: ["~", "null", "true"]` keeps three string tags through `fmt`.
- `tags: [a, b]` stays byte-identical through `fmt`.
- `check` raises no non-canonical finding for a list `fmt` just wrote.

### Plan

A flow-list item goes through a flow-context quoting rule instead of the block-scalar one. Reproduced on 8d797e8: an ADR with `tags: ["a, b", "x]"]`, after `living-docs fmt <bundle> --plain`, reads `tags: [a, b, x]]`: three tags in place of two, and a stray bracket.
