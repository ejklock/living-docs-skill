---
type: Issue
title: ParadeDB (Postgres + BM25) as a selectable db engine alongside SQLite
description: Add ParadeDB/Postgres with BM25 search as a second db engine behind the SearchIndex/DocStore ports, selectable by config, with the CRUD proven on both engines.
status: done
labels: [slice, database, paradedb, search]
blocked_by: [2, 7]
timestamp: 2026-07-16T00:00:00Z
---

## ParadeDB engine (BM25) alongside SQLite

Implements [ADR 0004](/adr/0004-db-engine-and-data-layer.md). **Slice 3** — the default,
server-grade search engine. Large slice: **sub-slice at dispatch** (engine trait wiring →
bm25 migration → BM25 search) under the 5-file / 6-AC cap.

### Objective link

Constitution (findability, at catalog scale) → ADR 0004 (ParadeDB default, SQLite opt-in,
SeaORM multi-backend) → this slice.

### Context manifest

- Read: `db-store` (slice 0002), SeaORM entities + migrations, ADR 0004.
- Seams touched: an engine-selection config; a ParadeDB migration creating the `bm25` index;
  a BM25 (`@@@`) raw-SQL search path parallel to the FTS5 one.
- Pattern: full-text search is raw SQL per engine; SeaORM covers only the normalized CRUD.

### Scope

Engine selectable (`sqlite` | `paradedb`). ParadeDB migration builds the `bm25` index; the
`SearchIndex` impl issues BM25 queries. The normalized CRUD runs on both engines. A
documented Docker path to bring up ParadeDB for local/CI use.

### Vertical Demo

- **Given** ParadeDB up (Docker) and a synced corpus, **When** I run
  `living-docs search --engine paradedb "supersede"`, **Then** ADR 0003 ranks in the BM25
  results.
- **Given** the SQLite engine, **When** I run the same search, **Then** it still ranks
  (no regression to slice 0002).

### Acceptance

- **Fitness function (dual-engine CRUD):** the normalized CRUD test suite runs green against
  both Postgres/ParadeDB and SQLite (engine-parameterized matrix). — `verify_by: test`
- **Fitness function (search):** a seeded corpus returns the expected top hit on ParadeDB
  (BM25) and on SQLite (FTS5). — `verify_by: command`
- db-mode with the SQLite engine runs with no server process. — `verify_by: test`

### Out of scope

No multi-project (slice 0005), no db-mode authoring (slice 0006). This slice adds an engine,
not new capabilities on top of search.

### Plan

Sub-slices at dispatch: (a) engine config + SeaORM Postgres backend; (b) `bm25` migration;
(c) BM25 search path + dual-engine matrix. Each demoable via the search command.

### Delivery note

Delivered as three slices (commits cdb06fd, cc6fd51, 3088e1f):

- **0004-A** — engine-selection seam: `connect(url: &str)` lets SeaORM infer the backend from
  the URL scheme; both `sqlx-sqlite` and `sqlx-postgres` are compiled in for runtime
  selection; a CLI `--engine {sqlite|paradedb}` flag maps to a URL (paradedb sources
  `$DATABASE_URL`). The web crate was adapted to the new signature (stays on the SQLite
  read-model; web-on-Postgres remains deferred to 0006).
- **0004-B** — per-engine migration + sync: the `records` table is created via SchemaManager
  (portable); the full-text index branches on the backend — SQLite keeps the FTS5 virtual
  table, ParadeDB creates `pg_search` + a `records_bm25` bm25 index. sync's rebuild is a
  no-op on Postgres (bm25 auto-indexes).
- **0004-C** — BM25 search + dual-engine matrix: `search` branches on the backend; the
  ParadeDB path uses a parameterized `paradedb.boolean`/`paradedb.match` BM25 query ranked by
  `paradedb.score`. A dual-engine test seeds a corpus through the real `sync()` path — the
  SQLite case runs in-memory with no server, the Postgres case runs when
  `LIVING_DOCS_TEST_PG_URL` is set.

Fitness functions green: dual-engine CRUD suite passes on both engines; a seeded corpus
returns ADR 0003 for `supersede` on BM25 (ParadeDB) and FTS5 (SQLite); db-mode on SQLite runs
with no server process. Verified live against ParadeDB 0.24.3.
