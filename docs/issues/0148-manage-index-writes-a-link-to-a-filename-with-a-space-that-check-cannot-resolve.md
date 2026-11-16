---
type: Issue
title: manage index writes a link to a filename with a space that check cannot resolve
description: Write a generated link to a filename with a space in angle brackets so check resolves it
status: open
timestamp: 2026-11-16T21:56:07Z
---

## 0148. manage index writes a link to a filename with a space that check cannot resolve

### Scope

`living-docs manage index` writes the link to a record whose filename holds a space as `[...](0002-valid record.md)`, without angle brackets, so `check` cannot resolve it and reports the record as an orphan (invariant 3). A record in a typed directory such as `adr/` with a space in its name therefore cannot pass `check`. Found by the coder and review of issue 0146 (reproduced with `docs/adr/0002-valid record.md`).

### Decision

- Write such links in the CommonMark angle-bracket form `<0002-valid record.md>`, which `check` already resolves, in every generated index row and supersede link.

### Acceptance

- `manage index` over a record named with a space writes a link `check` resolves, and `check` passes; existing indexes without spaces keep their bytes; a test proves both.
