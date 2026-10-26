---
type: Issue
title: check accepts a record path and fails on a one-way supersede link, and supersede keeps an earlier supersedes value
description: check given a record path checks the enclosing bundle scoped to that record; a supersedes or superseded_by link not answered by the other record is a violation; supersede appends to an existing supersedes value instead of replacing it
status: closed
timestamp: 2026-10-24T10:17:29Z
---

## 0066. check accepts a record path and fails on a one-way supersede link, and supersede keeps an earlier supersedes value

Found by the usage audit: agents pass a record file to `check` and get "bundle root not found" (five times in the transcripts), and `supersede` replaced an existing `supersedes` value, leaving a one-way link `check` did not see (memory lesson 75, living-docs 0.20.0).

### Scope

- `living-docs check <record.md>` resolves the enclosing bundle from the file's ancestors and checks it scoped to that record, as `--changed-files` does ([ADR 0062](/adr/0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md)); a bundle root still checks the whole bundle.
- `check` reports a violation when a record names another in `supersedes` or `superseded_by` and the other does not name it back.
- `supersede OLD NEW` keeps a value NEW's `supersedes` already carries and adds OLD to it.

### Decision

- A record path reuses the changed-files scoping; option not taken: a separate single-record mode.
- `supersedes` becomes a list only when it holds more than one value, so existing records stay byte-identical.

### Acceptance

- `check docs/adr/0002-x.md` from the repository root exits 0 on a conformant record and 1 naming that record on a broken one; a path outside any bundle exits 2 with the current message.
- A record whose `supersedes` names a record that does not name it back makes `check` exit 1 naming both; a correctly linked pair passes.
- After `supersede 0001 0003` and `supersede 0002 0003`, record 0003 names both 0001 and 0002 and `check` passes.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the one-way link check; tests red first.
2. Core: `supersede` keeping an earlier value.
3. CLI: a record path on `check`; tests through the binary.

### Outcome

All three parts are in. `check <record.md>` scopes the gate to that record. `supersede` keeps an earlier `supersedes` value (commit 89cad36). `check` now fails on a one-way link: living-docs-core/src/check/supersede_links.rs pairs records by type and number and reports a `supersedes` or `superseded_by` entry the other record does not answer, naming both records. Tests: cli/tests/check_supersede_links.rs (10 cases). cli/tests/check_core.rs had a fixture with an unanswered `superseded_by`; it is now two-way. Review approved. Test gaps it found are tracked in issue 0094, and a mixed `supersedes` list is tracked in issue 0095.
