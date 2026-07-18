---
type: Issue
title: projects root + multi-project ingestion and cross-project search
description: Introduce the projects table as the DB root, ingest multiple projects into one database, and scope/aggregate search and the web view by project.
status: done
labels: [slice, database, multi-project, schema]
blocked_by: [2]
tracker:
timestamp: 2026-07-16T00:00:00Z
---

## projects root + multi-project catalog

Implements [ADR 0005](/adr/0005-normalized-schema.md) (the `projects` root and
`project_id` FKs). **Slice 4** — the catalog dimension. Large slice: **sub-slice at
dispatch** (schema → ingestion → cross-project query/UI).

### Objective link

Constitution (findability extended to a multi-project catalog) → ADR 0005 (`projects` root,
`project_id` FK everywhere) → this slice.

### Context manifest

- Read: `db-store` schema + entities (slices 0002/0004), ADR 0005, the web crate (slice
  0003) for the project filter.
- Seams touched: `projects` table + `project_id` FK on records/relations/tags; a
  `db sync --project <slug>` ingestion assigning `project_id` at the DB boundary; search +
  web scoped/aggregated by project.
- Pattern: "project" is explicit only in db-mode; assigned at the DB boundary, derived from
  the repo.

### Scope

Add `projects` and `project_id` FKs. Ingest N repos into one database, each a `projects`
row. Search can scope to a project or span all; the web lists projects and filters by one.

### Vertical Demo

- **Given** two repos synced into one database, **When** I open the web and search
  `supersede`, **Then** results from both projects appear, each labeled by project, and a
  project filter narrows to one.
- **Given** a relation pointing at a record in another project, **When** ingestion runs,
  **Then** the FK constraint refuses the cross-project dangling edge (unhappy path).

### Acceptance

- **Fitness function (referential integrity):** inserting a `relations`/`record_tags` row
  pointing at a non-existent record is refused by an FK constraint. — `verify_by: test`
- Cross-project search test: a term present in two projects returns hits from both, labeled
  by project; a scoped search returns only one project's hits. — `verify_by: command`
- Every record/relation/tag row carries a valid `project_id`. — `verify_by: test`

### Out of scope

No db-mode authoring (slice 0006), no auth/tenancy isolation beyond `project_id`. File-mode
stays single-project (implicit).

### Plan

Sub-slices at dispatch: (a) `projects` schema + FKs + migration; (b) per-project ingestion;
(c) cross-project search + web filter. Each demoable via search/web.

### Delivery note

Delivered as four slices (commits 9b76eef, 39929ea, ac914cf, 1ffcdf0):

- **0005-A** — schema root: a `projects` table, `project_id NOT NULL` FK on `records` with
  `UNIQUE(project_id, path)`, and `relations`/`tags`/`record_tags` tables with FK
  constraints, via a new `CreateMultiProjectSchema` migration (the derived read-model's
  `records` table is recreated destructively, reusing the per-engine
  `create_search_index`/`drop_search_index` helpers). `connect()` enables SQLite
  `PRAGMA foreign_keys=ON` (Postgres enforces natively); `sync` assigns a single default
  project so every record carries a valid `project_id`.
- **0005-B** — per-project ingestion: `record.rs` parses `supersedes`/`superseded_by`/`tags`;
  `db sync --project <slug>` upserts the project by slug, scopes the delete/insert to that
  `project_id`, and populates `relations`/`tags`/`record_tags` in a two-pass resolve
  (targets resolved within the same project; dangling edges skipped, the FK is the
  structural backstop). The project-scoped `record_tags` delete goes through a portable
  SeaORM `in_subquery`.
- **0005-C1** — cross-project + scoped search: `SearchHit` carries a project label; per-engine
  SQL (FTS5 / BM25) JOINs `projects` for the label and adds an optional slug predicate when
  scoped; `living-docs search --project <slug>` narrows, else spans all with per-hit labels.
- **0005-C2** — web project filter: `db_store::list_projects` (portable SeaORM); the search
  page renders a project `<select>` filter, preserves the selection, and labels each result
  by project; a committed Playwright spec drives the browser gate.

Acceptance met: the referential-integrity fitness function is green (a dangling
`relations`/`record_tags` insert is refused on SQLite with `foreign_keys` ON); cross-project
search returns hits from both projects labeled, and a scoped search narrows to one, verified
live on **both** SQLite (FTS5) and ParadeDB (BM25); every record/relation/tag row carries a
valid `project_id`. The web project filter passed the browser gate against a two-project
read-model.
