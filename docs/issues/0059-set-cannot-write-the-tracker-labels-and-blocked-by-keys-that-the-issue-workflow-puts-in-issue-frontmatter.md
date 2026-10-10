---
type: Issue
title: set cannot write the tracker, labels and blocked_by keys that the issue workflow puts in issue frontmatter
description: The issue workflow says tracker metadata lives in frontmatter, but the CLI owns frontmatter and offers no way to write it.
status: open
timestamp: 2026-10-10T19:16:42Z
---

## 0059. set cannot write the tracker, labels and blocked_by keys that the issue workflow puts in issue frontmatter

Found adopting 0.21.0 in a downstream bundle (relent). `skills/living-docs/rules/issue-workflow.md` (lines 7 and 10) says `labels`, `blocked_by` and `tracker` live in the issue's frontmatter, and the project's own rules say to backfill the tracker number there after publishing. The `new issue` template carries none of the three, and `living-docs set` accepts only `status`, `description`, `owner` and `title`.

Reproduction: `living-docs set issue/0001 tracker 12` exits 2 with "'tracker' is not a settable field; expected one of status, description, owner, title". The same for `labels`. The only path left is a hand frontmatter edit, which the procedure itself calls a process error. In relent, 83 issues already carry `tracker`, 190 carry `labels` and 198 carry `blocked_by`.

### Scope

Add `tracker`, `labels` and `blocked_by` as settable keys, with the value shapes the existing records use (a number or URL for `tracker`, a list for `labels` and `blocked_by`), and make `fmt` keep them in canonical order. Kept: the three keys optional, so `new issue` need not emit them.

### Acceptance

- `living-docs set issue/NNNN tracker 12` writes the key and `check` stays OK.
- `set` writes `labels` and `blocked_by` as lists from a comma-separated value and rejects the keys on other doc types.
- `fmt --check` is clean after a `set` of each key.
- The error text for an unsupported key, `set --help` and issue-workflow.md name the new keys.
- `living-docs check docs` stays OK.
