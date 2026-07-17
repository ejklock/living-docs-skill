---
type: Issue
title: Findability — db sync builds a SQLite/FTS5 read-model and living-docs search queries it
description: Add a derived SQLite + FTS5 read-model built from the .md corpus and a `living-docs search` command that returns ranked hits — the first observable delivery of the North Star.
status: open
labels: [slice, search, findability]
blocked_by: [1]
tracker:
timestamp: 2026-07-16T00:00:00Z
---

## Findability — db sync + living-docs search

Implements [ADR 0003](/adr/0003-storage-backend-model.md) and
[ADR 0004](/adr/0004-db-engine-and-data-layer.md). **Slice 1** — the North Star's first
truth: answer "where did we decide X?" in seconds.

### Objective link

Constitution (findability, in seconds) → PRD/ADR 0003 (file-mode derived read-model) +
ADR 0004 (SQLite/FTS5 engine) → this slice.

### Context manifest

- Read: `living-docs-core` domain + `SearchIndex` port (from slice 0001),
  `living-docs-core/src/check/*` (frontmatter/body parsing to reuse),
  [ADR 0005](/adr/0005-normalized-schema.md) schema.
- Seams touched: new `db-store` crate implementing `SearchIndex` over SeaORM + a raw-SQL
  FTS5 index; a `db sync` code path that reads records via `DocStore` and writes the
  derived read-model; a `search` subcommand in `cli`.
- Pattern: the read-model is derived and rebuildable — never authoritative in file-mode.

### Scope

`living-docs db sync` parses the `.md` corpus (via the fs `DocStore`) into the SQLite
normalized read-model (records + FTS5 index over title/description/body).
`living-docs search "<query>"` runs the FTS5 query and prints ranked hits (path + title +
snippet). SQLite engine only.

### Vertical Demo

- **Given** a docs dir with the ADR trail, **When** I run `living-docs db sync` then
  `living-docs search "supersede"`, **Then** ADR 0003 appears in the ranked results with its
  path and a matching snippet.
- **Given** a query with no match, **When** I run `living-docs search "zzzznomatch"`,
  **Then** it prints no results and exits 0 (unhappy path).

### Acceptance

- Scripted CLI test: seed a known corpus → `db sync` → `search` for a term present in one
  record → assert that record ranks first and the no-match query returns empty. —
  `verify_by: command`
- `db sync` is idempotent: running it twice yields the same read-model (no duplicate rows). —
  `verify_by: test`
- The FTS5 virtual table exists and is populated after sync. — `verify_by: test`
- Complexity + clean-code + test-effectiveness standing ACs hold. — `verify_by: command`

### Out of scope

No web (slice 0003), no ParadeDB (slice 0004), no multi-project (slice 0005), no db-mode
authoring (slice 0006). Read-model is derived, not authoritative.

### Plan

`db-store` crate (SeaORM entities + FTS5 migration) → `db sync` over `DocStore` → `search`
subcommand → scripted findability test. Likely fits one pipeline slice; sub-slice if the
schema + sync + search exceed the file/AC cap.
