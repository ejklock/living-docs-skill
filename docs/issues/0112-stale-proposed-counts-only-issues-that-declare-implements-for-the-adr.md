---
type: Issue
title: stale-proposed counts only issues that declare Implements for the ADR
description: "Implements ADR 0079: the liveness check reads the Implements declaration on issue bodies instead of any link from the ADR."
status: closed
timestamp: 2026-11-12T16:14:04Z
---

## 0112. stale-proposed counts only issues that declare Implements for the ADR

Implements [ADR 0079](/adr/0079-stale-proposed-counts-only-an-issue-that-declares-it-implements-the-adr.md): `stale-proposed` counts an issue toward a Proposed ADR only when the issue's body declares `Implements` followed by a link to that ADR, so a historical citation of a closed issue in an ADR no longer reads as stale.

### Scope

- `living-docs-core/src/check/liveness.rs`: replace the ADR-side link scan with a scan of issue bodies for the declaration, keyed by the ADR each declaration resolves to.
- The `check` guide topic's description of `stale-proposed`.
- Kept: the advisory kind, its message, its exit-code posture and the seed-status and terminal-status rules.

### Acceptance

- A Proposed ADR whose body links a closed issue that does not declare it: no advisory.
- A Proposed ADR that links nothing, with a closed issue declaring `Implements [ADR NNNN](/adr/NNNN-slug.md)` for it: one `stale-proposed` advisory.
- The declaring issue open, or the ADR Accepted: no advisory.
- `implements` in lower case counts; `Implements ADR NNNN` without a link does not; an `Implements` link to another ADR does not.
- `living-docs check docs` stays green over this repository.
