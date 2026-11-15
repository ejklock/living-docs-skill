---
type: ADR
title: Every blocked_by finding is advice and none fails check
description: Supersede ADR 0086 so dangling numbers, self-references and cycles in blocked_by are advisories, after they still blocked commits in two real bundles
owner: Evaldo Klock
status: Accepted
supersedes: 0086
timestamp: 2026-11-15T23:09:26Z
---

# 0088. Every blocked_by finding is advice and none fails check

## Context

[ADR 0086](/adr/0086-blocked-by-fails-check-only-on-a-dangling-issue-number-a-self-reference-or-a-cycle-other-items-are-advice.md) kept three `blocked_by` findings as violations — a numeric item naming no issue, an issue listing itself, and a cycle — and reported every other item as advice. Rerun over the 28 living-docs bundles on the owner's machine after it landed, the pass still added 18 blocking violations, all numeric items naming no issue: 16 in agent-memory and 2 in jira-cli. Each one fails the pre-commit gate for any commit that touches the issue, although the item records a real intent (a dependency on work tracked elsewhere, or an issue not yet written). The owner asked that the dependency findings guide the author rather than stop the commit.

`check` remains the only gate ([ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md)); that is not in question. The question is whether a dependency finding is a record defect the gate must stop, or advice about planning data.

## Decision

We will report every `blocked_by` finding as an advisory of kind `blocked-by`, and none as a violation.

- A numeric item naming no issue of the bundle, an issue listing its own number, a cycle of issues (naming every member), and a non-numeric item are each reported as `blocked-by` advice and never change `check`'s exit status.
- Scalar comma lists, write-time acceptance, the named refusal of items YAML would read back as another value, and the renumber rewrite of numeric items are unchanged from ADR 0086.
- A derived wave plan, if one is built later, treats an unresolved number or a cycle as a reason not to place the issue, and says so; it never fails.

Rejected alternatives:
- **Keep dangling numbers and cycles as violations** (ADR 0086). Those are the certain defects, but 18 of them block commits in two real bundles today for data that is planning intent, not record integrity.
- **Make the whole pre-commit gate advisory and let only CI block.** It would remove blocking from the dependency findings too, but also from every record-integrity check the gate exists for, which ADR 0059 keeps as the tool's only enforcement.

## Consequences

**Easier / gained:**
- No commit is blocked by `blocked_by` content; authors see every dependency problem as advice in `check --advisories`.

**Harder / accepted trade-offs:**
- A cycle or a dangling dependency can be committed and stay until someone reads the advisories.
- Anything that consumes the dependency graph must tolerate unresolved items and cycles.

**Follow-ups:**
- An issue to move the three findings to the advisory channel and update the rules and tests.

## Verification

**Implementation impact:** `living-docs-core/src/check/dependencies.rs` and its tests, `cli/tests/check_dependencies.rs`, `skills/living-docs/rules/issue-workflow.md`, `skills/living-docs/rules/check.md`.

**Verification criteria:**
- A dangling number, a self-reference and a three-issue cycle each produce a `blocked-by` advisory and `check` exits 0; the cycle advisory names every member.
- Rerunning `check` over the 28 bundles adds no violation relative to 0.28.0.
