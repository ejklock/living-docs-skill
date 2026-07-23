---
type: Issue
title: Atlas create — db-mode authoring walking skeleton (mode guard, revision, transactional write+check)
description: The first Atlas write-path slice — a per-record revision column, a transactional write+check core verb, the file-mode/db-mode mode guard fitness function, and one browser-authorable create route — so the web front can author its first record end-to-end in db-mode only.
status: open
labels: [web, atlas, authoring, database]
blocked_by: [8]
timestamp: 2026-07-21T00:00:00Z
---

## Atlas create — db-mode authoring walking skeleton

Implements the first slice of [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
(Atlas is writable only in db-mode) and [PRD 0001](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
(the Atlas authoring wiki). Builds on [issue 0008](/issues/0008-three-pane-web-shell-with-metadata-panel-and-cmd-k-palette.md)'s
read-only three-pane shell. This is the walking skeleton for every Atlas write that
follows (edit, supersede, delete): once the mode guard, the `revision` column, and one
transactional write verb exist, the remaining verbs are additive.

### Objective link

Constitution → [PRD 0001](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
→ [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
→ this slice.

### Context manifest

- Read: `web` crate (routes, `lib.rs`, `views.rs` from issue 0008), `living-docs-core`'s
  `new` service and `DocStore` port, `db-store`'s `records` schema (ADR 0005/0007).
- Seams touched: a new `revision` column on `records` (db-store migration), a
  transactional write+`check` wrapper in `living-docs-core` that `new` (CLI) and Atlas's
  new create handler both call, and a `--backend`-style guard in `web` that only mounts
  mutating routes when the connected backend is db-mode.
- Pattern: db-mode is the sole authoritative store (ADR 0003); a browser write is never a
  second source of truth. The write and its `check` commit in one transaction — an invalid
  write never lands.

### Scope

- `db-store`: add a `revision: i64` column to `records` (default `1`, bumped on every
  committed write) via a migration; no behavior change for existing reads.
- `living-docs-core`: a transactional write+check verb — write the record, run `check`
  over the domain model inside the same transaction, commit only if `check` passes,
  bumping `revision`. `new` (CLI, `--backend db`) is refactored onto this verb so there is
  one write+check path, not two.
- `web`: one authoring route, `POST /record/{*path}` (or a dedicated `/new/{doc_type}`
  route — Coder's call at dispatch, consistent with existing route naming) that calls the
  same core verb, gated by a backend-mode check: file-mode mounts no mutating route at
  all; db-mode mounts it.
- A minimal creation form in the three-pane shell (doc type + title), reusing `views.rs`'s
  existing layout — no rich editor, no markdown-body authoring yet (that is a natural
  follow-up once this skeleton is proven, tracked separately, not blocking this slice).
- Browser spec under `tests/browser/` exercising: db-mode create succeeds and the new
  record is immediately visible in the nav tree and passes `check`; file-mode exposes no
  mutating route (405/404 fitness check).

### Vertical Demo

- **Given** Atlas connected to a db-mode project, **When** I submit the create form for a
  new ADR, **Then** the record appears in the nav tree, its page renders, and
  `living-docs --backend db check` reports it clean.
- **Given** Atlas connected to a file-mode project, **When** I attempt the same POST
  directly, **Then** the server returns not-available (no mutating route exists).

### Acceptance

- **Fitness function (mode guard):** with backend=file, the web exposes no mutating
  route (a test asserts every write endpoint is absent/refused); with backend=db, the
  create route authors successfully. — `verify_by: test`
- **Fitness function (doc-gate on write):** a create submission that would violate an
  invariant (e.g. empty required title) is rejected and no record is persisted; a valid
  submission commits and passes `check`. — `verify_by: browser`
- **No second source of truth:** exercising the create route never writes a `.md` under
  the docs dir except via an explicit `export`. — `verify_by: test`
- `revision` starts at `1` on a freshly created record and is queryable via the existing
  `record_meta` getter (issue 0008). — `verify_by: test`
- `living-docs check` and `cargo test --workspace` stay green. — `verify_by: command`

### Out of scope

Edit/supersede/delete (issues 0011/0012/0013). A rich markdown-body editor. Authn/authz
for a hosted deployment — ADR 0016 names this as a required follow-up ADR before Atlas is
exposed beyond a local/trusted deployment; this slice assumes a trusted, unauthenticated
local environment and must not be treated as production-ready without that ADR landing
first.

### Plan

Single vertical slice, sub-sliced at dispatch if it exceeds the 5-file/6-AC cap: (a)
`revision` column + migration; (b) transactional write+check core verb, `new` refactored
onto it; (c) `web` mode guard + create route + minimal form; (d) browser spec.
