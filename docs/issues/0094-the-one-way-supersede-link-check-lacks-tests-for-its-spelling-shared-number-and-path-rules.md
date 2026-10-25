---
type: Issue
title: the one-way supersede link check lacks tests for its spelling, shared-number and path rules
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T03:14:25Z
---

## 0094. the one-way supersede link check lacks tests for its spelling, shared-number and path rules

The one-way supersede link check (living-docs-core/src/check/supersede_links.rs) behaves correctly on the cases its review probed, but three of its rules have no test that fails when they break. The review of that slice proved each gap with a mutant that survived `cargo test -p living-docs --test check_supersede_links`.

### Scope

- cli/tests/check_supersede_links.rs.

### Acceptance

- A negative case with another spelling: 0003 `supersedes: 1` and 0001 with no `superseded_by` reports one violation naming `0001-d.md`. It fails when the target lookup accepts only four-character links.
- Two records sharing number 0001 and 0003 `supersedes: 0001` give no one-way-link violation, only the duplicate-number one. It fails when the lookup takes the first of the shared records.
- The path assertions match the bundle-relative path exactly, so dropping the bundle prefix strip fails a test.

### Plan

Add the three cases. The code does not change unless a case fails.
