---
type: Issue
title: projects root + multi-project ingestion and cross-project search
description: Introduce the projects table as the DB root, ingest multiple projects into one database, and scope/aggregate search and the web view by project.
status: open
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
