---
type: Issue
title: The HTML page keeps empty group headings under a filter and repeats a view's title
description: The text filter hides records but leaves their type and status headings, and an architecture view shows its title twice in its card
status: open
timestamp: 2026-11-16T04:49:09Z
---

## 0143. The HTML page keeps empty group headings under a filter and repeats a view's title

### Scope

The QA web walkthrough of 2026-11-16 over `manage html` output showed two readability issues: filtering for "digest" left the empty "ADR" and "Proposed" headings above the one matching issue, and an architecture view's card shows its title twice (the card title and the body's own title heading, which views carry without a number), which also made the title ambiguous for a locator.

### Decision

- Hide a type or status heading when the filter hides every record under it; drop a record body's leading heading when it repeats the card title.

### Acceptance

- Under a filter no heading without visible records shows; a view's title appears once per card; tests prove both and determinism holds.
