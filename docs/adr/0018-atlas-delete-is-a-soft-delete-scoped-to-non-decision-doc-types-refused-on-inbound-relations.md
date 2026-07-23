---
type: ADR
title: Atlas delete is a soft-delete, scoped to non-decision doc types, refused on inbound relations
description: Atlas's delete verb (ADR 0016, issue 0013) only applies to issue/concept records, never ADR/BDR/PRD/constitution; it soft-deletes via a deleted_at column rather than a hard row removal, and is refused outright when any inbound relation still points at the record.
status: Accepted
tags: [architecture, atlas, authoring, database, delete, web]
timestamp: 2026-07-21T00:00:00Z
---

# 0018. Atlas Delete Is a Soft-Delete, Scoped to Non-Decision Doc Types, Refused on Inbound Relations

## Context

[ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
lists `new`/`edit`/`supersede`/`delete` as the Atlas verbs mirroring the CLI, noting "delete
verb if absent." It is absent — `living-docs-core` has never had one — and the project's own
convention (`adr-conventions.md`, the constitution) is **supersede, never delete** for
decision records: an ADR/BDR is parked, its history kept, never removed. [Issue
0013](/issues/0013-atlas-delete-a-new-verb-with-no-cli-precedent.md) named three open
questions this ADR resolves before any delete code is planned: which doc types are
delete-eligible, hard-delete vs. soft-archive, and what happens to a record's inbound
`relations` on delete.

Atlas's motivation for `delete` (PRD 0001) is mundane: a mis-created draft, a duplicate OKF
`concept`, a stray issue — operational cleanup, not decision-log editing. Nothing in the PRD
or ADR 0016 asks for deleting an ADR/BDR/PRD; conflating "cleanup" with "erase a decision"
would be a scope creep this ADR forecloses explicitly.

Epistemic type: **judgment** (a safety/reversibility trade-off; no benchmark decides it).

## Decision

We will scope Atlas delete to **`issue` and OKF `concept` records only**, implement it as a
**soft-delete**, and **refuse it outright when the record has any inbound `relations`**.

1. **Delete-eligible types: `issue`, `concept`. Never `adr`/`bdr`/`prd`/`constitution`.**
   The existing supersede-never-delete convention for decision records is a **standing
   invariant**, not something Atlas's delete verb may bypass — those types have no delete
   route at all, mirroring how file-mode has no CLI delete today. `check`'s doc-gate stays
   the enforcement point: a delete attempt on a non-eligible type is refused before any
   write, the same shape as the mode-guard fitness function issue 0010 establishes for
   file-mode vs. db-mode.
2. **Soft-delete via a `deleted_at: Option<timestamp>` column**, not a hard row removal. A
   soft-deleted record is excluded from the nav tree, search (FTS index), and `index`
   generation by default, exactly as ADR 0009's visibility model already excludes
   non-matching records — this is a sibling filter on the same query paths, not new
   plumbing. The row, its body, and its `revision` history remain in the database:
   reversible by construction, auditable, and a browser click can never be an unrecoverable
   mistake. Hard delete (physical row removal) is explicitly rejected for this reason.
3. **Refuse delete when any inbound `relations` row still points at the record** (another
   record supersedes it, links to it, or tags reference it) — the same FK-refusal shape
   issues 0006/0012 already rely on for dangling-link prevention. The author must remove or
   redirect the inbound reference first. Cascade-delete is rejected: silently breaking
   another record's link is a worse failure mode than a blocked delete with a clear reason.
4. **Undelete is out of scope for this ADR** — the `deleted_at` column makes it a cheap,
   obvious follow-up (clear the timestamp) once there is a UI affordance for it; not
   required for issue 0013 to ship.

## Consequences

**Easier / gained:**
- Decision-log integrity (ADR/BDR/PRD/constitution) is structurally protected — the delete
  route never exists for those types, so no runtime check can be forgotten or bypassed.
- Soft-delete reuses ADR 0009's existing visibility-filter query shape; no new exclusion
  logic to invent and verify separately.
- Refusing on inbound relations reuses the same fitness-function shape (FK/relation
  refusal) issues 0006/0012 already established and tested — one pattern, not a new one.

**Harder / accepted trade-offs:**
- A record with any inbound reference cannot be deleted until that reference is cleaned up
  first — accepted: this is the same trade-off the dangling-link refusal already makes
  elsewhere, and cascade would be strictly worse.
- Soft-deleted rows accumulate in the database forever (no purge job). Accepted for now — a
  retention/purge policy is a real future need but not one issue 0013 has to solve; this ADR
  does not block it.
- Two different "hidden from view" mechanisms now exist (ADR 0009 visibility, this ADR's
  `deleted_at`) with similar-looking query filters. Accepted: they answer different
  questions (who may see it vs. is it still a live record) and conflating them into one flag
  would lose that distinction.

**Rejected alternative (the judgment critic):** *Hard-delete, unscoped to any doc type.* The
simplest possible verb — a real `DELETE FROM records`. It loses because a browser click is
one accidental confirm away from an unrecoverable loss of a decision record's history,
exactly the failure mode the project's supersede-never-delete convention exists to prevent;
scoping + soft-delete costs one column and one query filter to close that hole entirely.

**Follow-ups:**
- Issue 0013's implementation slice, sized against the 5-file/6-AC cap once this ADR is
  accepted.
- A future undelete affordance and a retention/purge policy — neither blocks 0013.

## Verification

**Implementation impact:** `db-store` (`deleted_at` column + migration, query filters on
list/search/index), `living-docs-core` (the delete verb, doc-type eligibility check,
inbound-relations refusal, all inside the transactional write+check wrapper from issue
0010), `web` (delete action gated the same way create/edit/supersede are — db-mode only,
mode-guard fitness function extended).

**Verification criteria:**
- **Type-scope fitness function:** a delete attempt on an `adr`/`bdr`/`prd`/`constitution`
  record is refused before any write; `issue`/`concept` records delete successfully. —
  `verify_by: test`
- **Soft-delete fitness function:** after a delete, the row still exists (readable by
  direct query) with `deleted_at` set, but is absent from the nav tree, search results, and
  `index` output. — `verify_by: test`
- **Relation-refusal fitness function:** deleting a record with at least one inbound
  `relations` row is refused with a clear error and no `deleted_at` is set; deleting a
  record with zero inbound relations succeeds. — `verify_by: test`

# References

[1] [ADR 0016 — Atlas makes the web a db-mode authoring front](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
[2] [ADR 0009 — document visibility model](/adr/0009-document-visibility-model.md)
[3] [Issue 0013 — Atlas delete, the open design question this ADR resolves](/issues/0013-atlas-delete-a-new-verb-with-no-cli-precedent.md)
