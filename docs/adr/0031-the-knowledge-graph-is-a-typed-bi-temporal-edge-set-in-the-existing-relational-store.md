---
type: ADR
title: The knowledge graph is a typed bi-temporal edge set in the existing relational store
description: The knowledge graph is typed bi-temporal edges widening the existing relations table in SQLite/ParadeDB — no graph engine, no probabilistic layer; queries are depth-bounded recursive CTEs.
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-08-05T20:18:36Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0031. The knowledge graph is a typed bi-temporal edge set in the existing relational store

## Context

The handoff that motivated [research 0003](/research/0003-provenance-aware-knowledge-graph-for-living-docs.md) proposed a knowledge graph with lineage between records and code, backed by a graph database (Neo4j was named) and a probabilistic inference layer (ProbLog) for confidence and impact scores. The research refuted both components and confirmed the graph itself. Three forces bound the decision. First, the determinism boundary (ADR 0001): no LLM and no heuristic inference inside the tool. Second, the db-store already holds the substrate — a `relations` table (today only `kind="supersede"`), records with `status`/`revision`/`deleted_at`, and FTS5 — so the graph is an evolution of an existing projection, not a new system. Third, the embedded graph-engine field collapsed in 2025 (Kuzu archived, CozoDB abandoned, IndraDB dormant, Neo4j JVM-only), while SQLite recursive CTEs carry two orders of magnitude of headroom at this scale.

## Decision

We will model the knowledge graph as a typed, bi-temporal edge set inside the existing relational store, in both engines (SQLite and ParadeDB). Concretely:

- **Nodes** are what already exists: records (and, in a later ADR, code symbols). No new node store.
- **Edges** live in the widened `relations` table. `kind` becomes a typed vocabulary (`supersedes`, `references`, `blocked_by`, `motivated_by`, plus doc-code kinds when ADR 0032 lands). Body markdown links between records — today validated by `check` and thrown away — are persisted as `references` edges at sync time.
- **Temporality** follows the SQL:2011 four-column pattern on each edge: `valid_from`/`valid_to` (event timeline — when the relation held true) and `system_from`/`system_to` (transaction timeline — when the store learned it). Supersede sets `valid_to` on the displaced edge; nothing is deleted. This is Graphiti's invalidation rule stripped of its LLM front-end: pure interval logic.
- **Queries** (`why`, `impact`, `as-of`) are depth-bounded recursive CTEs in the domain layer of `living-docs-core`, executed by db-store. Unbounded path enumeration is out.
- **Confidence and impact are deterministic**: status enums, graph reachability, and (if ever wanted) exponential recency decay computed at query time. There is no stored score.

Rejected alternatives: a dedicated graph engine (dependency risk without payoff at this scale; every embedded candidate is dead or license-encumbered) and a probabilistic-logic layer (no peer provenance system — W3C PROV, OpenLineage, Graphiti — uses one; ProbLog is Python-only with #P-complete exact inference; a stored probability would also smuggle non-reproducible judgment past the determinism boundary).

## Consequences

**Easier / gained:**
- Graph queries arrive as a schema migration plus CTEs — no new infrastructure, one deployable, both backends.
- Full history survives by construction: `as-of` reads reconstruct what the doc graph asserted at any past commit or date.
- The edge vocabulary gives `check` new deterministic invariants (dangling `blocked_by`, contradiction between `supersedes` edge and status).

**Harder / accepted trade-offs:**
- Bi-temporal columns complicate every edge write: sync must close intervals instead of overwriting rows.
- Typed kinds require a registry-driven vocabulary (ADR 0026 pattern) — a hand-synced enum would regress lesson 3973.
- CTE queries live in SQL, not a graph DSL; complex traversals stay verbose.

**Follow-ups:**
- ADR 0032 decides how doc-code edges are declared and anchored.
- Frontmatter tail keys that reference records (`blocked_by: [NNNN]`) get promoted from EAV storage to resolved edges at sync time.

## Verification

**Implementation impact:** `db-store/src/migration.rs`, `db-store/src/sync.rs`, `living-docs-core` (edge kind vocabulary + query domain), `cli` (query verbs).

**Verification criteria:**
- Syncing a bundle twice produces byte-identical edge sets (idempotence, extends the ADR 0001 fitness family).
- `living-docs supersede` closes the displaced edge's `valid_to` and never deletes a row; an `as-of` query dated before the supersede still returns the old edge.
- Edge kinds come from one registry row set; a test fails when a kind literal appears outside the registry.

# References

[1] [Research 0003 — provenance-aware knowledge graph for living-docs](/research/0003-provenance-aware-knowledge-graph-for-living-docs.md)
[2] [XTDB — Time in XTDB (SQL:2011 bitemporal pattern)](https://docs.xtdb.com/about/time-in-xtdb.html)
[3] [Zep: A Temporal Knowledge Graph Architecture for Agent Memory](https://arxiv.org/abs/2501.13956)
