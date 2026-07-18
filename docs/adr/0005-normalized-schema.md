---
type: ADR
title: Normalized DB schema — projects root, typed records with an EAV tail, typed relations
description: The database is fully normalized around a `projects` root with typed `records` columns for universal fields, an EAV `frontmatter_fields` table for the type-specific tail, and typed join tables for tags and relations — with identity chosen per DocType and check parity guaranteed against file-mode.
status: Proposed
supersedes:
superseded_by:
tags: [architecture, database, schema, normalization, eav, identity, check, multi-project]
timestamp: 2026-07-16T00:00:00Z
---

# 0005. Normalized DB schema — projects root, typed records with an EAV tail, typed relations

## Context

ADR 0003 makes db-mode authoritative and ADR 0004 picks the engines and ORM. This ADR
decides the **schema** — and it must be *fully normalized* (no JSON blob dumping), because
the database is now the store, not a throwaway index.

Three forces shape it:

1. **Multi-project.** A db-mode database is a catalog that can hold many projects (a central
   instance the web browses). Everything must hang off a `projects` root. (In file-mode a
   "project" is implicit — one repo, one `docs/` — so `project_id` is assigned only at the
   DB boundary.)
2. **Heterogeneous frontmatter.** Doc types carry different fields: an ADR has
   `status`/`supersedes`/`tags`; an OKF `concept` describing a table carries open,
   domain-specific keys. A column-per-field table would be a wide sea of NULLs; a JSON blob
   is vetoed.
3. **Dual identity.** ADRs/PRDs/BDRs use a permanent sequential `NNNN`; OKF concepts use a
   path-based `concept_id`. The schema must carry both.

And the `check` doc-gate — ADR 0001's whole point — must keep meaning when there is no file:
what is a "broken link" or "one-home" invariant inside a database?

## Decision

We will use a **fully normalized schema** rooted at `projects`:

- **`projects`** `(id, slug, name, root_path, …)` — the root everything references.
- **`records`** `(id, project_id→projects, doc_type, identity_kind, number NULL,
  concept_id NULL, slug, title, description, status, timestamp, body)` — **typed columns
  for the universal fields** shared by all doc types; `body` is stored for lossless export.
  `identity_kind` selects `number` (adr/prd/bdr) or `concept_id` (OKF concept) — the dual
  identity from ADR context.
- **`frontmatter_fields`** `(record_id→records, key, value, value_type)` — an **EAV** table
  for the type-specific scalar tail. Adding a doc type needs no migration; a hot key can be
  promoted to a typed `records` column later.
- **`tags`** `(id, project_id, name)` + **`record_tags`** `(record_id, tag_id)` — tags as a
  typed many-to-many, not EAV.
- **`relations`** `(id, project_id, from_record_id→records, to_record_id→records, kind)` —
  supersede links and cross-doc links as typed edges with **foreign keys**.
- A per-engine full-text index over `title`/`description`/`body` (`bm25` on ParadeDB, `fts5`
  virtual table on SQLite — ADR 0004).

The relationships that carry integrity (project membership, tags, supersede/link edges) are
**typed with FK constraints**; only the sparse type-specific scalars are EAV. This is the
"normalized, no blob" bar the requirement set.

**Check parity.** The `check` doc-gate is **one backend-agnostic implementation over the
domain model**, fed records/relations/bodies through the `DocStore` port — so a document
valid in file-mode is guaranteed valid in db-mode (same schema, supersede-chain, mermaid,
and body-link logic). In db-mode, **FK/NOT-NULL constraints are a second line of defense**
(a dangling link is refused at write time), but they do not replace the domain check.

## Consequences

**Easier / gained:**
- A real relational model: filter by `status`, walk supersede chains, list a project's docs,
  rank search — all as normal queries with referential integrity.
- New doc types cost no migration (EAV tail); known/hot fields stay typed.
- The DB enforces link/relationship integrity structurally; some check invariants become
  fail-fast at write.

**Harder / accepted trade-offs:**
- EAV fields are untyped text (`value_type` is advisory) and need a JOIN to read/filter —
  the accepted cost of open frontmatter without a blob. Validation of those values lives in
  the core, not the schema.
- Lossless `.md` export requires `body` + faithful frontmatter reconstruction from the typed
  columns + EAV — a round-trip obligation.
- Two identity kinds in one table (`number` xor `concept_id`) needs a check constraint.

**Follow-ups:**
- The web front (deferred ADR) reads this schema.
- Possible later ADR if a concept sub-domain (e.g. table columns) outgrows EAV and earns its
  own typed tables.

## Verification

**Implementation impact:** SeaORM entities + migrations for `projects`, `records`,
`frontmatter_fields`, `tags`, `record_tags`, `relations`, and the per-engine FTS index; the
lossless `.md` exporter; the backend-agnostic `check` over the domain model.

**Verification criteria:**
- **Fitness function (lossless round-trip):** author a doc, store it in db-mode, export back
  to `.md` — the export equals the canonical serialization of the original (byte-parity on a
  normalized form).
- **Fitness function (referential integrity):** inserting a `relations` row or `record_tags`
  row that points at a non-existent record is refused by an FK constraint (a test asserts
  the failure).
- **Fitness function (check parity):** the same corpus passes/fails the core `check`
  identically whether loaded from `fs-store` or `db-store`.
- A record has exactly one of `number` / `concept_id` per `identity_kind` (check-constraint
  test).

# References

[1] ADR 0003 (storage model), ADR 0004 (engine + data layer).
[2] [OKF concept_id — path-based identity](https://github.com/GoogleCloudPlatform/knowledge-catalog)
