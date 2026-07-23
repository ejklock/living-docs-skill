---
type: Issue
title: db-mode authoritative authoring — new/index/supersede/check on db-store, lossless .md export
description: Make the database an authoritative authoring backend (new/index/supersede/check run over db-store), with a lossless .md exporter and check parity against file-mode.
status: done
labels: [slice, database, authoring, schema]
blocked_by: [2, 5]
timestamp: 2026-07-16T00:00:00Z
---

> **Delivered** (commits `bd892d8` → `02b41b7`, ADR 0007). Sub-sliced A/B/C1/C2/D1/D2:
> A schema + EAV tail + dual identity; B canonical serializer + `db-store` `DocStore`
> read/list; C1 identity-from-path (number ← filename `NNNN`, `concept_id` ← path);
> C2 `DocStore::write` + backend-agnostic number allocation; D1 `check` reads via the
> `DocStore` port (backend-faithful); D2 `--backend fs|db` selector, authoritative `new`
> on db-store, and the `living-docs export` round-trip. **Deferred to a follow-up:**
> db-mode `index`/`supersede` (still fs-only) — the write port and canonical serializer
> they need already exist. The lossless round-trip and check-parity fitness functions are
> green end-to-end.

## db-mode authoritative authoring

Implements [ADR 0003](/adr/0003-storage-backend-model.md) (db-mode authoritative) and
[ADR 0005](/adr/0005-normalized-schema.md) (check parity, lossless export). **Slice 5** —
the second authoring surface. Large slice: **sub-slice at dispatch** (db `DocStore` write →
export → parity).

### Objective link

Constitution (db-mode is an authoritative authoring backend, config-selected) → ADR 0003 +
ADR 0005 → this slice.

### Context manifest

- Read: `db-store` (slices 0002/0004/0005), `living-docs-core` `DocStore` port + services
  (`new`/`index`/`supersede`/`check`), ADR 0003, ADR 0005.
- Seams touched: `db-store` implements the write side of `DocStore`; number allocation via
  `SELECT MAX+1`; a lossless `.md` exporter; the backend-agnostic `check` run over the
  db-loaded domain model.
- Pattern: exactly one backend active per project (config); no bidirectional sync.

### Scope

With `--backend db`, `new`/`index`/`supersede` write to the database authoritatively. A
`living-docs export` materializes records back to conformant `.md`. `check` runs over the
domain model loaded from `db-store`, identical to file-mode.

### Vertical Demo

- **Given** db-mode configured, **When** I run `living-docs --backend db new adr "x"` then
  `living-docs --backend db check`, **Then** the record is stored in the DB and passes the
  same check as file-mode.
- **Given** a db-authored corpus, **When** I run `living-docs export`, **Then** the emitted
  `.md` passes the file-mode `check` (round-trip).
- **Given** a supersede that would dangle, **When** I wire it, **Then** the FK refuses it
  (unhappy path) and `check` also reports it.

### Acceptance

- **Fitness function (lossless round-trip):** author in db-mode → `export` → the `.md`
  equals the canonical serialization of the record (byte-parity on the normalized form). —
  `verify_by: command`
- **Fitness function (check parity):** the same corpus passes/fails `check` identically
  whether loaded from `fs-store` or `db-store`. — `verify_by: test`
- A record has exactly one of `number` / `concept_id` per `identity_kind` (check
  constraint). — `verify_by: test`

### Out of scope

No web authoring (ADR 0006 keeps the web read-only). No live backend toggle — switching is a
one-time migration.

### Plan

Sub-slices at dispatch: (a) db `DocStore` write (new/index/supersede + number allocation);
(b) lossless exporter; (c) check parity fs↔db. Each demoable via CLI + export.
