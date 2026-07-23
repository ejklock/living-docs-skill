---
type: ADR
title: Atlas makes the web a db-mode authoring front, superseding web read-only
description: The Living Docs Atlas web front becomes writable, but only in db-mode where the database is the single authoritative store (ADR 0003) — so there is still no second source of truth and no cross-backend sync; in file-mode the web stays read-only, and intra-db concurrency is single-store optimistic (a per-record revision precondition), never a merge.
status: Accepted
supersedes: 0006
tags: [architecture, atlas, authoring, concurrency, db-mode, source-of-truth, web]
timestamp: 2026-07-20T15:09:34Z
---

# 0016. Atlas Makes the Web a db-mode Authoring Front, Superseding Web Read-Only

## Context

The [Living Docs Atlas PRD](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
turns the web front from a read-only viewer into a **multi-project authoring wiki** — create,
edit, supersede, delete in the browser. That directly reverses the central decision of
[ADR 0006](/adr/0006-web-read-only-axum.md), which kept the web **read-only** precisely to
avoid "reintroducing a second source of truth and the sync/conflict problem ADR 0003 killed."
So the write path cannot be bolted on; it must be decided against the locked storage model.

The binding constraints are two prior locked decisions, not open questions:

- **[ADR 0003](/adr/0003-storage-backend-model.md)** made the `DocStore` backend
  **config-selected and mutually exclusive**: *file-mode* (`.md` in git is the source of
  truth; `search`/web read a **derived, never-authoritative read-model**) **or** *db-mode*
  (the database **is** the authoritative store). Because exactly one backend is authoritative
  per deployment, "there is no bidirectional sync and no conflict to resolve." ADR 0003
  explicitly **rejects** co-authoritative equal-weight adapters. (Note: the "two interchangeable
  adapters / both authoritative" phrasing still living in `CLAUDE.md` is stale relative to
  ADR 0003; this ADR treats 0003 as the SSOT.)
- **[ADR 0007](/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md)** gave
  db-mode a lossless, byte-stable `export` (db → `.md`) and `check` parity across backends.

The naive framing of this decision — "define the fs↔db write-authority / sync-conflict
contract" — is therefore a **category error**: ADR 0003 already dissolved cross-backend sync
by exclusivity. The real questions Atlas forces are (a) *in which mode* browser authoring is
even coherent, and (b) what the *single-store* concurrency contract is when several writers
(Atlas tabs, a CLI db-mode writer, multiple users) hit the one authoritative DB.

Epistemic type: **judgment** (no benchmark decides it; it is a safety/simplicity trade-off
under the git-native, single-authoritative-store constraint). The critic applied below names
the strongest rejected alternative.

## Decision

We will make the Atlas web front **writable only in db-mode, where the database is the single
authoritative store (ADR 0003); in file-mode the web stays read-only.**

1. **Authoring requires db-mode.** In db-mode the DB is the sole source of truth, so a browser
   write introduces **no second source of truth and no cross-backend sync** — the exact hazard
   ADR 0006 feared is avoided by *scoping authoring to the authoritative backend*, not by
   forbidding authoring. In **file-mode** the web continues to render the derived read-model
   read-only (ADR 0003: that projection is never authoritative, so it must not be written
   through); file-mode authoring stays CLI + git-native `.md`.

2. **Writes go through `living-docs-core`, gated by `check` inside the write transaction.**
   Atlas handlers invoke the same core verbs as the CLI (`new`/edit/`supersede`/`delete`);
   the write and its `check` run in one SQLite transaction. An invalid write (broken link,
   bad frontmatter, size, Mermaid) **never commits** — the doc-gate is enforced on write, in
   the browser exactly as on disk. This preserves ADR 0006's "web cannot drift from the CLI."

3. **Concurrency is single-store optimistic, never a merge.** Each record carries a monotonic
   `revision` (bumped on every committed write). A browser write submits the `base_revision`
   it read; if the stored revision has moved, the write is **rejected** ("changed underneath
   you — reload"), never silently overwritten or auto-merged. This is ordinary optimistic
   concurrency against one store — categorically different from the cross-backend merge ADR
   0003 rejected.

4. **`.md` in a db-mode deployment is an export artifact, not a co-authoring surface.**
   `living-docs export` (ADR 0007) is a one-way materialization (db → `.md`) for git/PR/diff/
   backup. Hand-editing an exported `.md` and re-importing **normalizes** it (ADR 0007); the
   file is never a second authoritative store. A project that wants git-native `.md` authoring
   uses file-mode — and there the web is read-only, by decision 1.

5. **Server-rendered Rust/axum reusing `living-docs-core` carries forward** from ADR 0006 (no
   SPA, one language, one build); only the read-only clause is reversed.

## Consequences

**Easier / gained:**
- The Atlas authoring vision (PRD 0001) becomes buildable without reopening the storage model:
  authoring lands in the one authoritative backend, so ADR 0003's "no sync" guarantee holds.
- The doc-gate runs on every browser write; a bad edit cannot be persisted.
- "Which one wins" has a crisp answer per mode — file-mode: `.md`/git; db-mode: the DB — and a
  real intra-store concurrency rule (revision precondition), with no distributed-merge machinery.

**Harder / accepted trade-offs:**
- **Authoring is a db-mode-only capability.** A file-mode (git-native) project does not get
  in-browser authoring; it authors via CLI. Accepted: the alternative (writing through a
  non-authoritative file-mode read-model) is exactly the second-source-of-truth ADR 0003/0006
  forbid.
- A hosted, writable, multi-project surface needs **authn/authz** and durable DB operations
  (backup, migration) — new operational surface, spawned as follow-ups below.
- Every record needs a `revision` column and every write path must honor the precondition — a
  small schema + handler cost on top of ADR 0007's model.

**Rejected alternative (the judgment critic):** *Let file-mode web author by writing `.md` and
rebuilding the read-model.* This is the strongest tempting option (it would give git-native
projects browser authoring). It loses because it makes the web a writer of the git tree from a
process that only holds a derived projection — reintroducing exactly the divergence/sync
problem ADR 0003 dissolved, and risking clobbering out-of-band git/editor/PR edits. Optimistic
concurrency against a *projection* cannot be sound because the projection is not the truth.

**Follow-ups:**
- ADR: **authn/authz** for the hosted Atlas surface (who may read / who may edit).
- ADR (gated): the **ontology substrate** (extend SQLite + JSON-LD projection vs embed
  Oxigraph), opened only when the evidence-gate opens — research
  [/research/0002-ontology-tooling-codegraph-docs-bridge.md](/research/0002-ontology-tooling-codegraph-docs-bridge.md).
- ADR (later): whether Atlas's independent deploy cadence justifies splitting the `web` front
  from the CLI (the locked monorepo's documented "reconsider when" trigger).

## Verification

**Implementation impact:** `db-store` (a `revision` column + optimistic-write check),
`living-docs-core` (transactional write+`check`, `revision` bump, delete verb if absent),
`web` (authoring routes for create/edit/supersede/delete, backend-mode guard), and browser
specs under `tests/browser/`.

**Verification criteria:**
- **Mode guard (fitness function):** with backend=file, the web exposes **no** mutating route
  (an HTTP test asserts every write endpoint returns not-available); with backend=db, the same
  routes author successfully. — `verify_by: test`
- **Doc-gate on write (fitness function):** a browser edit that violates an invariant is
  rejected and the stored record is unchanged; a valid edit commits and passes `check`. —
  `verify_by: browser`
- **Optimistic concurrency:** two writes from the same `base_revision` — the first commits, the
  second is rejected with a stale-revision error; no silent overwrite or merge. — `verify_by: test`
- **No second source of truth (db-mode):** exercising the Atlas write path never writes a `.md`
  under the docs dir except via an explicit `export`; the DB is the only store mutated by a
  browser write. — `verify_by: test`
- **Supersede parity:** superseding via the Atlas UI leaves both records linked and conformant,
  identical to the CLI path (ADR 0001 fitness function). — `verify_by: test`

# References

[1] [ADR 0003 — storage backend is config-selected and mutually exclusive](/adr/0003-storage-backend-model.md)
[2] [ADR 0006 — the web view is read-only (superseded by this ADR)](/adr/0006-web-read-only-axum.md)
[3] [ADR 0007 — db-mode authoring data model and lossless export contract](/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md)
[4] [PRD 0001 — Living Docs Atlas](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
