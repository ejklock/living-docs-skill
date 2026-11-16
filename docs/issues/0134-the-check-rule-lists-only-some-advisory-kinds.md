---
type: Issue
title: The check rule lists only some advisory kinds
description: skills/living-docs/rules/check.md lists SIZE, MOVED-SOURCE and LIVENESS but not OWNER or HEADING, which check also emits
status: closed
timestamp: 2026-11-15T22:42:58Z
---

## 0134. The check rule lists only some advisory kinds

### Scope

`skills/living-docs/rules/check.md` (about lines 18-27) lists the advisory kinds an author can see from `check`: SIZE, MOVED-SOURCE and LIVENESS. `living-docs-core/src/check/kinds.rs` also registers OWNER and HEADING, which the rule does not mention, so a reader of the guide cannot tell what those advisories mean. Found by the review of issue 0133 slice S1, which adds BLOCKED-BY to the list; the two missing kinds predate it.

### Decision

- The rule lists every kind in `kinds::ALL`, one line each; a test compares the two so they cannot drift again.

### Acceptance

- check.md names every advisory kind `check --help` lists, and a test fails when a kind is added without its line.

### Outcome

- `skills/living-docs/rules/check.md` lists every kind in `kinds::ALL` with its backticked label (the LIVENESS line was reworded), and `cli/tests/check_rule_kinds.rs` fails when a kind has no line.
