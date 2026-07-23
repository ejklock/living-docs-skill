---
type: Issue
title: Atlas delete — a new verb with no CLI precedent
description: ADR 0016 names delete as an Atlas verb "if absent" from the CLI — it is absent — so this slice needs a small ADR deciding what "delete" means for a living-docs record before any code lands, given the project's standing supersede-never-delete convention for decision records.
status: open
labels: [web, atlas, authoring, database, needs-adr]
blocked_by: [10]
timestamp: 2026-07-21T00:00:00Z
---

## Atlas delete — a new verb with no CLI precedent

[ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
lists `new`/`edit`/`supersede`/`delete` as the Atlas verbs mirroring the CLI, noting
"delete verb if absent." It is absent: `living-docs-core` has no delete service today, and
the project's own ADR/BDR convention is explicitly **supersede, never delete** — a decision
record is parked, not removed. This slice cannot start with a plan the way 0010–0012 could,
because the design question is open: **is browser delete for every doc type, or only for
records that were never meant to be permanent (e.g. a draft issue, an OKF concept, a stray
duplicate), and does it hard-delete or soft-archive?**

### Objective link

Constitution (supersede, never delete, for decision records) → [PRD 0001](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
→ [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
→ this slice — **blocked on a new ADR, not blocked on 0011/0012's code.**

### Context manifest

- Read: the constitution's / `adr-conventions.md`'s supersede-never-delete rule,
  `living-docs-core`'s services module (confirms no `delete` function exists), issues
  0010–0012 (the transactional write+check wrapper delete would also need to run inside).

### Open design question (resolve via `tradeoff-analysis` + a new ADR before coding)

1. Which doc types, if any, are hard-deletable (an OKF `concept`? a draft `issue`?) versus
   which must stay supersede-only forever (ADR/BDR — the decision log must never lose a
   parked record)?
2. Is a "delete" a real row removal, or a soft-delete (a status/visibility flip plus
   exclusion from the nav tree and search) that keeps the audit trail ADR 0009's
   visibility model already gives us?
3. What happens to a record's inbound `relations` (supersede links, cross-doc links) on
   delete — refuse if any exist (safer, mirrors the FK-refusal pattern issues 0006/0012
   already rely on), or cascade?

### Scope (provisional — pending the ADR above)

- A new ADR deciding the above three questions.
- Once decided: a `living-docs-core` delete verb (through the same transactional
  write+check wrapper as create/edit/supersede), a `web` delete action gated the same way
  create/edit/supersede are (db-mode only, mode-guard fitness function extends to this
  route too), and a browser spec.

### Acceptance

- The new ADR is written, reviewed, and accepted before any delete code is dispatched.
  — `verify_by: inspection`
- (Post-ADR) Whatever delete semantics the ADR locks are covered by the same class of
  fitness functions as 0010–0012: mode guard, doc-gate on write, and — if relations are
  refused on a non-empty inbound edge set — a test proving that refusal. — `verify_by: test`

### Out of scope

Coding a delete verb before the ADR lands. Bulk/multi-record delete.

### Plan

1. Run `tradeoff-analysis` (or a direct ADR) on the three open questions above with the
   project owner.
2. Only then plan the implementation slice the way 0010–0012 were planned, sized against
   the 5-file/6-AC slicing cap.
