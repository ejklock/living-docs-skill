# Maintenance Invariant (no-drift)

The rule that keeps docs *living*: **no structural change ships without its doc, and no doc exists without being indexed.** Code, docs, and indexes stay in sync within the *same* change — never "documented later."

## The invariant, stated for the project guide

Paste a project-specific version of this into the project guide (`CLAUDE.md` / `README.md`) as a mandatory section:

> **Maintenance rule (mandatory).** Whenever the project structure changes (new directory, moved/renamed files, a new top-level component) or a new doc is generated:
>
> 1. **Document the change** — create or update the relevant file under `docs/` (as an OKF concept: frontmatter with a non-empty `type`).
> 2. **Update the indexes** — add the doc to the bundle-root `docs/index.md` and to its directory `index.md` listing.
> 3. **Update architecture docs** — whenever the schema, data flow, module layout, or a component relationship changes, update the relevant diagram(s) in the same change. Diagrams must never drift from the code.
> 4. **Update the context index** — whenever a new module or domain concept is named, add it to the vocabulary so naming stays consistent across code, docs, and reviews.
>
> No structural change ships without its doc, and no doc exists without being indexed.

Adapt the specifics (diagram tooling, context index location) to the project. The invariant does not change.

## What counts as a structural change

- New directory, module, or top-level component.
- Moved, renamed, or deleted files that other docs reference.
- Schema change, new data flow, changed module boundary.
- A new domain concept being named in code.
- A new doc of any type (ADR/PRD/issue/research) being created.

## The checklist (run before declaring a task done)

- [ ] Did this change add/move/rename a file another doc points to? → update those pointers.
- [ ] Did it name a new concept? → add it to the context index.
- [ ] Did it change schema/data flow/module layout? → update the architecture diagram(s).
- [ ] Did it create a new doc? → give it OKF frontmatter (`type`), then link it from its directory `index.md` *and* the bundle-root `docs/index.md`.
- [ ] Did it change a decision? → supersede the ADR via frontmatter `status`/`superseded_by` (don't rewrite it).
- [ ] Do all index links still resolve? (prefer bundle-relative `/…` links)

## The instrument (don't eyeball what a script can check)

The orphan, broken-link, untyped-doc, and supersede checks in the list above are mechanical —
run them, don't re-read prose. `living-docs check <docs/>` validates them and exits non-zero
on any violation; wire it into the project's CI/quality gate so a structural PR with a docs
violation cannot merge. *A constraint without an instrument is a vibe.* The stale-diagram and
one-home-per-fact checks have no sound oracle and stay a reviewer judgement.

## Why same-change, not later

A doc update deferred to "later" is a doc update that never happens — and the gap between code and docs is exactly where the next person gets misled. Coupling the doc to the change that caused it is the only mechanism that survives turnover and time pressure. The reviewer enforces it: a structural PR with no doc delta is incomplete.

Same-change is also the cheap way when the author is an agent: in the same change the model still holds the full context of what it just did, so the doc costs only its output tokens (a terse record is small — see `size-targets.md`). Deferring docs to a later "documentation session" re-pays the entire input context cold before a single line is written.
