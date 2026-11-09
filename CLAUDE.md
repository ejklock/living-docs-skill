# CLAUDE.md — living-docs

Project guidance for any agent working in this repo. These are **hard rules**, not
suggestions. They override default behavior. When a rule and a convenience conflict, the
rule wins.

## What this project is

`living-docs` is the deterministic layer of Living Docs authoring (see `docs/adr/0001`).
A Rust CLI owns the mechanical, template-fillable steps (`new`, `set`, `supersede`,
`index`, `check`, `fmt`, `read`) so the authoring model never pays tokens for them. There is **no LLM
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

### 5. Responsibility split + file-size ratchet (issue 0028)

Maintainability is a gate, not an advisory. Layout rules for all Rust code:

- **One responsibility per module.** `cli/src/main.rs` holds only clap wiring and
  dispatch; every verb lives in `cli/src/commands/<verb>.rs`. New verbs are BORN in
  their own module — never added to `main.rs`.
- **Hard cap: 30 lines per function.** Enforced by clippy `too_many_lines`
  (`clippy.toml` threshold 30, `-D warnings` in CI). This is the primary clarity
  instrument — Clean Code's limit is about functions, not files.
- **Hard cap: 300 lines per `.rs` file** (sibling `tests.rs` has its own 300 cap).
  Enforced by `scripts/check-file-size.sh` (ratchet: a grandfathered file may only
  shrink; growing one fails the check).
- **Sibling test files.** A `#[cfg(test)]` module over ~100 lines moves to
  `<module>/tests.rs` via `mod tests;` — private access preserved, production file clean.
- Deep modules still win over many shallow files (rule 2): split by responsibility,
  never by line-count alone — the file cap is the backstop, not the design driver.

## Architecture

A **modular monolith** organized hexagonally, cut to the authoring core by ADR 0059:

```
living-docs-core   — domain + the DocStore port, no I/O
adapters:
    fs-store       → .md files in git
fronts:
    cli            → depends on core, injects fs-store
```

### Locked decisions

- **Single repository, Cargo workspace.** `core`, `fs-store` and `cli` share one domain and
  ship together. The hexagonal port is the extraction seam: a new consumer is born as a
  workspace front (ADR 0033), never a new repo, until it needs its own deploy cadence.
- **The `.md` tree in git is the only backend (ADR 0059).** There is no read-model, no
  search index, no web front and no publication path in the workspace. They return only
  as workspace fronts when a consumer needs cross-project search or an independently
  deployed surface; `grep` and `read --topic` answer the search question until then.
- **Ten verbs, one gate.** `new`, `set`, `supersede`, `index`, `check`, `fmt`,
  `read`, `guide`, `install`, `uninstall`, plus the `completions` shell-completion
  utility (ADR 0060), which is not a verb of the contract. `check` at commit and in CI is the only
  enforcement; there is no write-time hook.
- **One authoring path.** `new` scaffolds the numbered file with frontmatter and heading
  filled and every body section as a `{{SLOT: hint}}`; the author edits the body; an
  unfilled slot fails `check`.

## Working conventions

- Every decision expensive to reverse (a verb's contract, the registry, a gate) gets an ADR
  via `living-docs new adr "..."` **before** code. Decide, then implement. A cheap choice
  lives in the issue that carries the work.
- `living-docs check` must pass over `docs/` — it is the doc-gate.
- **Docs authoring is CLI-first and gate-enforced (ADR 0059).** Write ONLY the body below
  the closing `---` of a record; numbering, frontmatter, supersede links, and index rows
  come from `living-docs new`/`set`/`supersede`/`index`/`fmt`. `.githooks/pre-commit`
  runs the doc-gate before every commit and CI runs it again; a hand-written record fails
  there.
- Conventional Commits; ticket ID when one exists. No AI attribution in commit messages.
- Never bypass a failing hook with `--no-verify`; fix the cause.
