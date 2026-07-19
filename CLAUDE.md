# CLAUDE.md — living-docs

Project guidance for any agent working in this repo. These are **hard rules**, not
suggestions. They override default behavior. When a rule and a convenience conflict, the
rule wins.

## What this project is

`living-docs` is the deterministic layer of Living Docs authoring (see `docs/adr/0001`).
A Rust CLI owns the mechanical, template-fillable steps (`new`, `brief`, `index`,
`supersede`, `next`, `check`) so the authoring model never pays tokens for them. There is **no LLM
inside the tool** — it is deterministic by construction.

## Hard rules

### 1. No comments in code

The only permitted comments are **language docblocks** documenting a type, its params, and
its return — plus the rare non-obvious **why** (an invariant, a gotcha, a spec reference).

- Rust: `///` (item docs) and `//!` (module docs) only.
- **Forbidden:** any comment that restates *what* the code does, section banners
  (`// --- discovery ---`), TODO/FIXME left in a merged change, and commented-out code.
- If a block needs a comment to be understood, that is a signal to **extract a
  well-named function** instead of explaining it.

Rationale: this repo already regressed on decorative banners twice (lessons 3514, 3606).
Names and structure carry intent; comments drift and lie.

### 2. Self-explanatory code + complexity budget

- Intention-revealing names. Guard clauses and early returns over nesting.
- Cyclomatic ≤ 10 (≤ 8 for new functions); cognitive complexity kept low.
- Prefer deep modules with narrow interfaces over many shallow functions.

### 3. Tests assert behavior, not implementation

- Every runtime/logic change ships with tests **in the same pass**. No patch without tests.
- Tests assert observable behavior. A test that only mirrors the implementation is a smell.
- The fitness functions in ADR 0001 (`new` output passes `check`; `index` is idempotent;
  `supersede` leaves both records linked and conformant) stay green.

### 4. Determinism boundary

The tool never writes rationale prose, never chooses a doc's epistemic type, never resolves
which alternative wins. Those belong to the authoring model. Everything the tool does must
be reproducible from its inputs.

## Architecture

Target shape is a **modular monolith** (start here; split into crates only when a real
seam demands it), organized hexagonally:

```
living-docs-core   — domain + ports (traits), no I/O
    ports:  DocStore (read/write records) · SearchIndex (FTS5)
adapters:
    fs-store   (LocalFileStorage)  → .md files
    db-store   (DatabaseStorage)   → SQLite normalized + FTS5
fronts:
    cli   → depends on core, injects an adapter
    web   → axum server on core, reads the db-store projection
```

### Locked decisions

- **Single repository, Cargo workspace (monorepo).** `core`, `cli`, and `web` share one
  domain and ship together. They live as members of one workspace, not separate repos:
  domain changes stay atomic (one PR, one CI), with no cross-repo version coordination.
  The hexagonal ports are the extraction seam — splitting into separate repos later is
  cheap *because* the boundary already exists. **Reconsider only when** a front needs an
  independent deploy cadence or separate ownership; until then, splitting adds release
  friction for no gain.
- **Two interchangeable adapters.** `fs-store` and `db-store` are equal-weight backends
  behind `DocStore`, selected by config/flag. Because both can be authoritative, the
  **sync/conflict contract between them must be defined explicitly** (an ADR before code) —
  do not leave "which one wins" implicit.
- **Web = Rust/axum reusing `living-docs-core`.** One language, one build, no model drift
  between CLI and web. Web reads the db-store projection.
- **CLI search defaults to the DB backend**, FTS5-powered (`living-docs search "..."`),
  with an explicit sync step to (re)build the projection from records.
- **Delivery sequence:** S1 extract `living-docs-core` + ports (refactor, no new behavior)
  → S2 `db-store` + FTS5 + `search` → S3 web. Each slice is vertical and demoable.

## Working conventions

- Every architectural fork (adapter sync contract, DB schema, web surface) gets an ADR via
  `living-docs new adr "..."` **before** code. Decide, then implement.
- `living-docs check` must pass over `docs/` — it is the doc-gate.
- Conventional Commits; ticket ID when one exists. No AI attribution in commit messages.
- Never bypass a failing hook with `--no-verify`; fix the cause.
