---
type: ADR
title: SQLite store for minted links
description: Persist links in a single-file SQLite database so they survive restarts.
status: Accepted
supersedes: 0001
superseded_by:
tags: [storage]
timestamp: 2026-06-20T00:00:00Z
---

# 0002. SQLite store for minted links

## Context

[ADR 0001](0001-in-memory-store.md) chose an in-memory map, which loses every link on
restart — a direct violation of the constitution's "links are permanent" non-negotiable.
Phase 1 is single-region and low-volume, so a full database server is overkill, but
durability is mandatory.

## Decision

We will persist links in a single-file **SQLite** database with one `links` table
(`code` primary key, `target_url`, `created_at`).

## Consequences

**Easier / gained:**
- Links survive restarts; the permanence non-negotiable holds.
- Still zero infrastructure — one file on disk, no server to operate.

**Harder / accepted trade-offs:**
- Single-writer; not suitable for the multi-region Phase 2. A later ADR will supersede
  this when that force arrives (one-way door deferred, not pre-solved).

**Follow-ups:**
- None for Phase 1.

## Verification

<!-- Optional block (Q7): closes the doc → implement → verify loop. Each criterion is
     checkable, so an implementing agent and a reviewer share one definition of done. -->

**Implementation impact:** `src/store.py` (the storage adapter), `src/schema.sql`.

**Verification criteria:**
- A link minted before a process restart still resolves after it (durability test).
- The `links` table has `code` as the primary key (a duplicate `code` insert fails).
- Fitness function: `tests/test_store_durability.py` fails if storage loses data across a
  restart.

# References

[1] [SQLite — When to use](https://www.sqlite.org/whentouse.html)
