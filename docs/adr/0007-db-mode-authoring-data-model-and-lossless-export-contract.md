---
type: ADR
title: db-mode authoring data model and lossless export contract
description: Fixes the three data-model decisions ADR 0005 deferred — the frontmatter tail store, the dual-identity columns, and the canonical lossless-export contract — so db-mode authoring (issue 0006) can be planned.
status: Proposed
supersedes:
superseded_by:
tags: [database, schema, authoring, export, identity, eav]
timestamp: 2026-07-17T14:41:11Z
---

# 0007. db-mode authoring data model and lossless export contract

## Context

[Issue 0006](/issues/0006-db-mode-authoring.md) makes the database an authoritative
authoring backend: with `--backend db`, `new`/`index`/`supersede` write to `db-store`, a
`living-docs export` materializes records back to conformant `.md`, and `check` runs over the
domain model loaded from the DB, identical to file-mode. [ADR 0003](/adr/0003-storage-backend-model.md)
established db-mode as authoritative (not a projection); [ADR 0005](/adr/0005-normalized-schema.md)
drew the normalized schema but **explicitly deferred** three data-model decisions to whoever
planned db-mode authoring. Issue 0005 shipped only the `projects` root, `project_id` FKs, and
the `relations`/`tags` tables — the current `records` shape is `(id, project_id, path,
doc_type, identity Option<String>, title, description, body)`.

Three forks blocked planning 0006, each a data-model one-way-door analyzed with the user via
`tradeoff-analysis`:

1. **The frontmatter tail** — how to store the per-DocType frontmatter keys that are not
   universal typed columns (e.g. `status`, `labels`, `tracker`, OKF-specific keys).
2. **Dual identity** — `number` (NNNN, sequential, for `adr`/`prd`/`bdr`) vs `concept_id`
   (path-based, for OKF concepts), discriminated by `identity_kind`.
3. **Lossless export** — what invariant "byte-parity lossless" is over, and what the canonical
   `.md` serialization is.

The binding forces: **portability** across SQLite (FTS5) and ParadeDB/Postgres (BM25) over
SeaORM with no raw per-engine SQL ([project lesson 3696](): raw placeholders are not
translated per backend); **losslessness** (every frontmatter key + value + order must
round-trip); **type-safety** for the number-allocation path (`SELECT MAX(number)+1` per
`(project, doc_type)`); and ADR 0001's standing idempotency fitness function ("regenerating
twice yields no diff").

## Decision

**1. Frontmatter tail → a normalized EAV table.** A `frontmatter_fields(record_id, key,
value, ordinal)` table holds every frontmatter key that has no universal typed column, with
`ordinal` preserving key order for a byte-stable round-trip. It is plain relational (portable
across both engines, no engine-specific JSON operators). We accept the EAV tax (a join and a
row per field, stringly-typed values) in exchange for per-field queryability and fidelity to
ADR 0005's normalized intent. Fields that later need typing "graduate" out of the tail into a
real column.

**2. Dual identity → two typed nullable columns + a discriminator.** `records` gains
`number: Option<i32>`, `concept_id: Option<String>`, and `identity_kind` (replacing the single
polymorphic `identity` column). `number` as a real integer keeps `SELECT MAX(number)+1`
allocation and ordering clean. The "exactly one of `number`/`concept_id`, matching
`identity_kind`" invariant is enforced **primarily in the domain model** (the backend-agnostic
`check` over `DocStore` is authoritative, per the CLAUDE.md check-parity convention); a DB
`CHECK` constraint is added as defense-in-depth **only if** it can be expressed portably via
SeaORM — never as raw per-engine SQL.

**3. Lossless export → a canonical, byte-stable round-trip.** "Lossless" means the canonical
form is a fixed point: `parse(export(record)) == record` and `export(parse(md)) == md` for
canonical `md`. `export` emits typed fields in a fixed template order (`type, title,
description, status, supersedes, superseded_by, tags`, …), then the EAV tail by its `ordinal`,
then trailing fields (`timestamp`, `tracker`). It does **not** attempt to reproduce the
arbitrary key order of a hand-authored file (in db-mode the record is born from `new`, never
had an original file). This extends ADR 0001's idempotency guarantee to the exporter.

## Consequences

**Easier / gained:**
- 0006 is now plannable: schema tail (EAV + dual-identity columns) → authoritative
  `DocStore` write with clean MAX+1 allocation → canonical exporter → `check` parity.
- Every decision is portable by construction (relational EAV, integer `number`, domain-model
  XOR, canonical text serialization) — no raw per-engine SQL, honoring lesson 3696.
- The canonical export is byte-stable, so `export` output is diff-free in git and passes the
  same `check` as file-mode.

**Harder / accepted trade-offs:**
- EAV pays a join + a row per field and stringly-typed values; reconstructing frontmatter is
  an ordered aggregate over `ordinal`. Accepted because the tail is queried per-field but
  never in a hot path.
- Two mostly-null identity columns (a mild normalization smell) instead of one; the XOR lives
  in the domain model, so a DB-only reader gets weaker enforcement unless the portable `CHECK`
  lands.
- Export reproduces the canonical form, not arbitrary hand-authored key order — a non-issue
  for db-mode-authoritative records, but it means importing a non-canonical `.md` and
  re-exporting normalizes it.

**Follow-ups:**
- Issue 0006 sub-slices: (a) schema tail + dual-identity migration + authoritative `DocStore`
  write with MAX+1 allocation; (b) canonical lossless exporter; (c) `check` parity fs↔db.
- ADR 0005 is confirmed (EAV) and detailed (identity columns), not superseded; this ADR adds
  the export contract on top of ADR 0001 + ADR 0003.

## Verification

**Implementation impact:** `db-store` (new `frontmatter_fields` entity + migration; `records`
identity columns), `living-docs-core` (`DocStore` write side, number allocation, the canonical
serializer + `export` command, the backend-agnostic `check`), `cli` (`--backend db`,
`export`).

**Verification criteria:**
- **Lossless round-trip (fitness function):** author in db-mode → `export` → the emitted `.md`
  equals the canonical serialization of the record (byte-parity on the normalized form);
  exporting twice yields no diff. — `verify_by: command`
- **Check parity (fitness function):** the same corpus passes/fails `check` identically whether
  loaded from `fs-store` or `db-store`. — `verify_by: test`
- **Identity invariant:** every record has exactly one of `number` / `concept_id` matching its
  `identity_kind`; a violation fails `check` in the domain model on both backends. —
  `verify_by: test`
- **Tail fidelity:** a record whose frontmatter carries non-typed keys round-trips those keys,
  values, and order through `frontmatter_fields.ordinal`. — `verify_by: test`
- **Portability:** every new query works identically on SQLite and ParadeDB with no raw
  per-engine SQL (SeaORM query builder or per-engine-branched statements only). —
  `verify_by: test`
