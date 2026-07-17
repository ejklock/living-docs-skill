---
type: Issue
title: ParadeDB (Postgres + BM25) as a selectable db engine alongside SQLite
description: Add ParadeDB/Postgres with BM25 search as a second db engine behind the SearchIndex/DocStore ports, selectable by config, with the CRUD proven on both engines.
status: open
labels: [slice, database, paradedb, search]
blocked_by: [2, 7]
tracker:
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
