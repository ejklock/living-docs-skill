---
type: ADR
title: Storage backend is config-selected, mutually exclusive, and both modes authoritative
description: The DocStore backend is chosen by CLI config — file (markdown, default) or database — never both at once; each is authoritative in its mode, so there is no bidirectional sync and no source-of-truth conflict.
status: Proposed
supersedes:
superseded_by:
tags: [architecture, storage, ports, source-of-truth, configuration]
timestamp: 2026-07-16T00:00:00Z
---

# 0003. Storage backend is config-selected, mutually exclusive, and both modes authoritative

## Context

ADR 0002 gives the core a `DocStore` port with a first `fs-store` (`.md`) adapter. We now
want a database backend too, and the shape of the relationship between the two is the
load-bearing decision.

A tempting framing — "two interchangeable adapters, both live" — is a trap. If both a file
store and a database store were authoritative at the same time, a doc could be written to
one and not the other, and we would owe a **bidirectional sync/merge contract**: when
`.md` and the DB disagree, who wins? That is expensive (timestamps, vector clocks, locks)
and error-prone, and it quietly breaks ADR 0001's premise that `.md` in git is the
authoring surface (git blame, PR review, prose diffs).

The actual need is simpler: some users/projects want plain markdown in git (the default,
git-native flow); others want to author into a database (queryable, multi-project, web).
These are **two ways to use the tool, never simultaneously** — a deployment/config choice,
not a runtime dual-write.

## Decision

We will make the `DocStore` backend a **config-selected, mutually exclusive** choice, with
**file (markdown) as the default**:

- **file-mode (default):** documents are authored as `.md` files under `docs/` in the
  project cwd (overridable via `--docs-dir`). No project-slug prefix — the repo *is* the
  project. The `.md` files are the source of truth; git owns history.
- **db-mode:** documents are authored into the database. In this mode the database is the
  source of truth (see ADR 0004 for the engine, 0005 for the schema).

**Both modes author** — db-mode is a real authoritative backend, not a read-only mirror.
Because exactly one backend is active per project/deployment, the two are never
co-authoritative, so **there is no bidirectional sync and no conflict to resolve**. The
core services (`new`/`index`/`supersede`/`check`/`search`) run port-driven and behave
identically over whichever backend is selected.

Search reconciles the two cleanly: in **file-mode**, `search`/web read a **derived
read-model** the tool builds from the `.md` (a projection — rebuildable, never
authoritative); in **db-mode**, they read the authoritative store directly. The database
therefore plays two roles behind one port — derived index in file-mode, authoritative
store in db-mode.

This **extends** ADR 0001 (which implicitly assumed files only); it does not supersede it —
the CLI still owns the deterministic layer, now over a selectable backend.

## Consequences

**Easier / gained:**
- No sync/merge machinery, ever — exclusivity makes conflict structurally impossible.
- The default stays git-native and self-contained, honoring ADR 0001; the DB is opt-in.
- One core, two backends, identical behavior — the value of the ADR 0002 port seam.

**Harder / accepted trade-offs:**
- db-mode being authoring (not just projection) means every core service must work over the
  DB, and the `check` invariants must have DB semantics (resolved in ADR 0005).
- Switching a project's backend is a one-time migration, not a live toggle — accepted.
- "Interchangeable equal-weight adapters" is explicitly rejected (the sync cost it implies
  buys nothing the exclusive-config model doesn't already give).

**Follow-ups:**
- ADR 0004 (engine + data layer) and ADR 0005 (schema, identity, check parity).

## Verification

**Implementation impact:** a backend-selection config on the CLI (`file` | `db`), the
adapter-wiring seam in `cli/`, and the file-mode derived read-model builder.

**Verification criteria:**
- **Fitness function (exclusivity):** with backend=file, a run touches no database; with
  backend=db, a run touches no `.md` under the docs dir. A test asserts each mode's I/O
  surface.
- **Fitness function (parity):** a document authored in file-mode and the same document
  authored in db-mode both pass the identical core `check` (ADR 0005 parity).
- Default with no config is file-mode (a test invoking the CLI with no backend flag writes
  `.md`).

# References

[1] ADR 0001 — the deterministic-layer premise this decision extends.
