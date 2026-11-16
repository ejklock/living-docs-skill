---
type: Issue
title: check accepts the placeholder description that new writes
description: Make check refuse the placeholder description that new scaffolds, or stop new from writing one.
status: open
timestamp: 2026-10-27T13:00:11Z
---

## 0105. check accepts the placeholder description that new writes

### Scope

`living-docs new` writes the `description` frontmatter key as a placeholder, `<One sentence — the change and its motivation.>` for an issue and `<One sentence — the decision and its scope.>` for an ADR. `check` fails an unfilled `{{SLOT}}` in the body but accepts this placeholder. About ten records in `docs/` still carry it, for example ADR 0041, issues 0029, 0030 and 0084 to 0090, and issues 0100 and 0101.

### Decision

- The owner chose that `check` reports a description equal to a doc-type template placeholder as an unfilled-slot violation, while `new` keeps writing the placeholder, as [ADR 0085](../adr/0085-check-fails-a-record-whose-description-is-still-its-template-placeholder.md) records. Not taken: `new` leaving the key out until `set` writes it; keeping it as it is.
- Fill the description of every record that still carries the placeholder in the same change, so the bundle stays green.

### Acceptance

- A record whose `description` is still the placeholder fails `check`, or `new` no longer writes one. The deciding cases each have a test: a placeholder, a filled description, and a description that only quotes the placeholder text.
- `living-docs check` passes over `docs/`.
