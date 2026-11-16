---
type: Issue
title: read --all does not show which listed records are retired
description: The read view gains a per-record status or retired marker, so a reader of --all output can tell history from live records.
status: open
timestamp: 2026-11-16T04:27:19Z
---

## 0121. read --all does not show which listed records are retired

`read records --all` lists retired records next to live ones, and neither the text nor the JSON view tells them apart: the view carries no status, so a closed issue prints exactly like an open one. Found by the review of issue 0079.

### Scope

- The `read` view (text and JSON) shows each listed record's status, or a retired marker, so a reader of `--all` output knows which records are history.
- The JSON contract only gains a field.

### Acceptance

- `read records --all --json` carries a status (or retired flag) per record; a closed issue and an open issue differ in it.
- The text view marks a retired record under `--all`.
- `living-docs check docs` stays OK.
