---
type: ADR
title: db-mode runs on ParadeDB by default with SQLite opt-in, over SeaORM
description: The database backend defaults to ParadeDB (Postgres + BM25) with SQLite (+FTS5) as an opt-in engine, accessed through SeaORM for portable normalized CRUD while full-text search is hand-written raw SQL per engine.
status: Proposed
supersedes:
superseded_by:
tags: [architecture, database, paradedb, postgres, sqlite, seaorm, search, data-layer]
timestamp: 2026-07-16T00:00:00Z
---

# 0004. db-mode runs on ParadeDB by default with SQLite opt-in, over SeaORM

## Context

ADR 0003 makes db-mode an authoritative backend. This ADR decides the **engine** and the
**Rust data layer** behind it.

Two full-text engines are in play. **ParadeDB** (a Postgres extension, `pg_search`) gives
Elasticsearch-grade BM25 ranking and real multi-project, concurrent, server-hosted access —
the right shape for a central catalog served by the web front. **SQLite + FTS5** is
embedded (a single file, no server), keeping the tool self-contained.

Choosing a default has a cost. ParadeDB is Postgres, so db-mode with ParadeDB requires a
**running Postgres server** — a runtime service dependency the file-mode default and ADR
0001's "self-contained binary" do not have. The binary itself stays self-contained (a
Postgres *client* in one file); it is the db-mode *deployment* that gains the dependency.

On the Rust side, the data layer must span both engines. `sqlx`'s compile-time query
checking binds to one live database via `DATABASE_URL`, making a dual-engine build (Postgres
*and* SQLite) painful (offline mode, two schemas, feature flags). A runtime query-builder
avoids that. Full-text search cannot be abstracted by any ORM regardless — BM25 (`@@@`,
`bm25` index) and FTS5 (`MATCH`, virtual table) are different SQL that must be written by
hand per engine; the ORM's value is only the normalized CRUD and migrations.

## Decision

We will make **ParadeDB (Postgres + BM25) the default db-mode engine**, with **SQLite
(+FTS5) an opt-in** engine, both behind the ADR 0002 `DocStore` / `SearchIndex` ports.

We will use **SeaORM** as the data layer: async-native (fits the axum web front),
multi-backend by design (Postgres and SQLite) with runtime query building (no compile-time
single-DB binding), and unified migrations via `sea-orm-migration`. The **full-text search
paths are raw SQL per engine** (BM25 for ParadeDB, FTS5 `MATCH` for SQLite), issued through
SeaORM's raw-statement API; only the normalized CRUD (`projects`, `records`, `relations`,
`tags` — see ADR 0005) goes through SeaORM entities.

This **extends** ADR 0001: the CLI binary stays self-contained, but db-mode with the default
engine consciously takes a Postgres runtime dependency. The self-contained property is
preserved for file-mode and for db-mode-on-SQLite.

## Consequences

**Easier / gained:**
- Best-in-class search (BM25) and real multi-project/concurrent access for the catalog+web
  scenario, out of the box.
- SQLite opt-in keeps a fully embedded, zero-server path for local/dev use.
- One entity model + migration set spans both engines; search stays honest (native per
  engine) instead of a lowest-common-denominator abstraction.

**Harder / accepted trade-offs:**
- The default db-mode requires provisioning Postgres — an infra cost every db-mode user
  pays unless they opt into SQLite.
- Two search implementations (BM25, FTS5) and two index-creation migrations to maintain.
- SeaORM adds ORM indirection over hand-written SQL; accepted for the multi-backend payoff.

**Follow-ups:**
- ADR 0005 (the normalized schema these entities and indexes materialize).
- Deferred: a documented Postgres/ParadeDB provisioning path (Docker) for db-mode.

## Verification

**Implementation impact:** a `db-store` crate implementing `DocStore` + `SearchIndex` over
SeaORM; per-engine migrations (a `bm25` index for ParadeDB, an `fts5` virtual table for
SQLite); engine selection in db-mode config.

**Verification criteria:**
- **Fitness function (dual-engine CRUD):** the normalized CRUD test suite runs green against
  **both** Postgres/ParadeDB and SQLite (a test matrix parameterized by engine).
- **Fitness function (search):** a seeded corpus returns ranked results for a query on both
  engines — BM25 on ParadeDB, FTS5 on SQLite.
- db-mode with the SQLite engine runs with no server process (embedded-path test).

# References

[1] [ParadeDB / pg_search](https://github.com/paradedb/paradedb)
[2] [SeaORM](https://www.sea-ql.org/SeaORM/)
