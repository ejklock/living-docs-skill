---
type: ADR
title: Extract a hexagonal living-docs-core inside a Cargo workspace
description: Restructure the standalone `cli/` crate into a Cargo workspace whose `living-docs-core` holds the domain plus storage/search ports, leaving the CLI a thin front — so a database backend and a web front can reuse one domain without model drift.
status: Accepted
tags: [architecture, core, hexagonal, okf, ports, workspace]
timestamp: 2026-07-16T00:00:00Z
---

# 0002. Extract a hexagonal living-docs-core inside a Cargo workspace

## Context

ADR 0001 established the `living-docs` CLI as the deterministic layer of doc authoring,
shipped as a single Rust crate under `cli/` that operates **directly on `.md` files**.
There is no storage abstraction: `commands/` and `check/` reach the filesystem inline.

Two new capabilities are now planned (see ADRs 0003–0005): a **database backend** users can
author into, and a **web front** for querying the corpus. Both need the same domain logic
the CLI already has — frontmatter parse/serialize/validate, number allocation, supersede
wiring, index generation, and the conformance `check`. Duplicating that logic across a CLI,
a DB layer, and a web server would guarantee drift: three subtly different notions of "what
a valid ADR is". The domain must live in **one** place, behind interfaces the fronts and
storage backends depend on — the hexagonal (ports-and-adapters) shape.

The deterministic core logic is not invented here. The upstream Open Knowledge Format
reference implementation (`GoogleCloudPlatform/knowledge-catalog`, Apache-2.0) already
factors this cleanly in Python: `bundle/paths.py` (concept_id ↔ path with segment
validation), `bundle/document.py` (frontmatter `parse`/`serialize`/`validate`, required
keys), and `bundle/index.py` (grouping/sorting skeleton). We port that logic to Rust — but
**drop its `synthesize_description` call**, which invokes an LLM (`gemini-flash-latest`) to
write directory descriptions. ADR 0001 forbids any LLM inside the tool; the deterministic
skeleton is portable, the generative call is not.

## Decision

We will restructure the repo into a **Cargo workspace** (single repo, modular monolith)
with these members:

- **`living-docs-core`** — the domain: `Record`, `Frontmatter`, `DocType`,
  `SupersedeChain`, `Link`, `Diagram`, plus the services (`new`, `index`, `supersede`,
  `next`, `check`, `search`). It defines two **ports** as traits:
  - `DocStore` — read and write records (the authoritative store).
  - `SearchIndex` — full-text query.
  The core depends on **no** adapter, no front, and no I/O concretion — only on the traits.
- **`fs-store`** — the first `DocStore` adapter: `.md` files (today's behavior).
- **`cli`** — a thin front that parses args, selects an adapter, and calls core services.

`living-docs-core`'s deterministic functions are a Rust port of the OKF reference
implementation's `paths` / `document` / `index` logic (Apache-2.0), with attribution in a
`NOTICE` file and the ported modules' `//!` docs, and with the LLM synthesis path
deliberately omitted.

This is a **refactor**: no new user-facing behavior. It is the precondition for ADRs
0003–0005 (storage model, engine, schema) and a later web front (deferred ADR).

**Remaining a monorepo is deliberate** (not separate repos): `core`, `cli`, and the future
`web` share one domain and change atomically in one PR. The ports are the extraction seam if
independent deploy cadence ever justifies a split.

## Consequences

**Easier / gained:**
- One definition of the domain; CLI, DB backend, and web can never drift on "what a valid
  ADR is" because they all call the same core.
- Adding a backend (DB) or a front (web) becomes "implement a trait" / "depend on core",
  not "reimplement the domain".
- The port seam makes the core unit-testable without touching the filesystem.

**Harder / accepted trade-offs:**
- Workspace ceremony and an indirection (trait dispatch) the single crate did not have.
- The refactor must preserve behavior exactly — a parity risk mitigated by the fitness
  function below.
- A dependency-direction rule (core must not depend on adapters/fronts) is now a property
  to enforce, not just intend.

**Follow-ups:**
- ADR 0003 (storage backend model), ADR 0004 (db engine + data layer), ADR 0005 (schema).
- A later ADR for the web front (axum reusing core), deferred until after S2.

## Verification

**Implementation impact:** new workspace root `Cargo.toml`; new crates
`living-docs-core/`, `fs-store/`; `cli/` reduced to arg-parsing + adapter wiring; a
`NOTICE` file recording the OKF Apache-2.0 attribution.

**Verification criteria:**
- The existing CLI integration tests (`cli/tests/*.rs`) pass **unchanged** after the
  extraction — the behavior-parity fitness function proving the refactor changed structure,
  not behavior.
- `cargo test -p living-docs-core` exercises the domain with an in-memory `DocStore`, no
  filesystem.
- **Fitness function (dependency direction):** `living-docs-core`'s manifest declares no
  dependency on `cli`, `fs-store`, or any adapter; a build/arch assertion fails if the core
  ever imports an adapter or front.

# References

[1] [OKF reference implementation — GoogleCloudPlatform/knowledge-catalog](https://github.com/GoogleCloudPlatform/knowledge-catalog) (Apache-2.0)
