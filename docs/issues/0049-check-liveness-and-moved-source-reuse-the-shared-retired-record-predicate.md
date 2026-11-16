---
type: Issue
title: check liveness and moved-source reuse the shared retired-record predicate
description: The liveness and moved-source checks decide retirement through DocTypeSpec::is_retired, so the rule has one copy
status: closed
timestamp: 2026-09-23T13:53:23Z
---

## 0049. check liveness and moved-source reuse the shared retired-record predicate

[ADR 0063](../adr/0063-authoring-advisories-skip-retired-records.md) made `DocTypeSpec::is_retired` the one definition of a retired record, but two `check` modules still carry their own copy of the rule: `check/liveness.rs` (`is_terminal_issue_status`) and `check/moved_source.rs` (`is_closed_dependent` with `is_terminal_for_type`). The copies agree with the predicate today; two copies of one rule drift apart the day the registry grows a status.

### Scope

- `check/liveness.rs` and `check/moved_source.rs` decide retirement through `DocTypeSpec::is_retired`, and their private copies go away.
- Kept: every finding both modules emit, byte for byte, on every input.

### Decision

A dependent whose status is `Superseded` but whose `type` is missing or unregistered stays closed in `moved-source`, as today, even though `is_retired` needs a registry row to answer. The option not taken was to let such a record count as live, which would change a finding for a malformed record that `check` already rejects.

### Acceptance

- `is_terminal_issue_status` and `is_terminal_for_type` no longer exist; no `check` module decides retirement on its own, except the missing-type fallback this issue's Decision keeps.
- The existing liveness and moved-source tests pass unchanged, including a `Superseded` dependent with no registered type.
- `living-docs check` over `docs/` reports the same findings before and after.
