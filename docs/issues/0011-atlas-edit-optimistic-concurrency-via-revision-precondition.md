---
type: Issue
title: Atlas edit — optimistic concurrency via a revision precondition
description: Adds the Atlas edit route on top of the create-slice's write+check plumbing, enforcing the ADR 0016 single-store optimistic-concurrency contract — a submitted base_revision that has moved is rejected, never silently overwritten or merged.
status: open
labels: [web, atlas, authoring, database, concurrency]
blocked_by: [10]
timestamp: 2026-07-21T00:00:00Z
---

## Atlas edit — optimistic concurrency via a revision precondition

Implements the edit verb + concurrency clause of [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
("Concurrency is single-store optimistic, never a merge"). Builds directly on
[issue 0010](/issues/0010-atlas-create-db-mode-authoring-walking-skeleton.md)'s `revision`
column and transactional write+check verb — this slice adds the precondition check, not a
second write path.

### Objective link

Constitution → [PRD 0001](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
→ [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
→ issue 0010 → this slice.

### Context manifest

- Read: issue 0010's transactional write+check verb and `revision` column, `web`'s record
  page (issue 0008) which already renders `record_meta`.
- Seams touched: the edit form submits the `revision` it read alongside the body/frontmatter
  edits; the core write verb gains a `base_revision: Option<i64>` precondition — when
  present, the write commits only if the stored revision still matches, else it is
  refused with a "changed underneath you — reload" error and no commit.
- Pattern: ordinary single-store optimistic concurrency (a WHERE-clause precondition on
  the update), never a distributed merge.

### Scope

- `living-docs-core`: extend the write+check verb with an optional `base_revision`
  precondition; a mismatch returns a distinct, user-facing "stale revision" error without
  writing anything.
- `web`: an edit form on the record page (pre-filled from the current record, carrying its
  `revision` as a hidden field) and a `POST`/`PUT` edit route wired to the precondition-aware
  write verb; on a stale-revision rejection, the page reloads the current record and shows
  the conflict message instead of silently discarding the user's edit.
- Browser spec: a normal edit commits and bumps `revision`; a simulated stale edit (two
  browser contexts editing the same record, first commits, second submits its now-stale
  `revision`) is rejected with the conflict message and the stored record is unchanged.

### Vertical Demo

- **Given** a record open in Atlas, **When** I edit its title and submit, **Then** the
  change commits, `revision` increments, and the page reflects the new title.
- **Given** two browser sessions with the same record open, **When** the first submits an
  edit and the second then submits using its now-stale `revision`, **Then** the second
  submission is rejected with a reload prompt and the first edit's content is what is
  stored.

### Acceptance

- **Optimistic concurrency (fitness function):** two writes from the same `base_revision`
  — the first commits, the second is rejected; no silent overwrite or merge. —
  `verify_by: test`
- **Doc-gate on write:** an edit that would violate an invariant is rejected and the
  stored record is unchanged; a valid edit commits and passes `check`. — `verify_by: browser`
- **Stale-edit UX:** the browser spec confirms a rejected edit surfaces the conflict
  message and reloads the current (server) content, rather than losing the user's input
  silently. — `verify_by: browser`
- `living-docs check` and `cargo test --workspace` stay green. — `verify_by: command`

### Out of scope

Supersede/delete (issues 0012/0013). Any merge or diff-based conflict resolution — a
rejected edit is always a hard reject, per ADR 0016's rejected-alternative analysis.

### Plan

Single vertical slice: (a) `base_revision` precondition on the write+check verb; (b) edit
form + route in `web`; (c) browser spec covering both the happy path and the stale-revision
rejection.
