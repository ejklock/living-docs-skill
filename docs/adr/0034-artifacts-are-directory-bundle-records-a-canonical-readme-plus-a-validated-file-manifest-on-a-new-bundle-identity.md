---
type: ADR
title: "Artifacts are directory-bundle records: a canonical README plus a validated file manifest on a new Bundle identity"
description: Add a third Identity variant, Bundle, for directory-per-record artifacts whose canonical README lists the files it references, and register an artifact doc type on it with a check-validated manifest.
status: Proposed
timestamp: 2026-08-07T14:36:44Z
---

# 0034. Artifacts are directory-bundle records on a new Bundle identity

## Context

After [ADR 0026](/adr/0026-a-single-doctype-registry-replaces-nine-hand-synced-enumerations-and-research-and-constitution-enter-as-rows.md) and [ADR 0027](/adr/0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md), every doc type is one row in the `DOC_TYPES` table. The `new`, `index`, `check`, and web consumers derive a type's directory, frontmatter, template, and rules from that row, so no site hand-codes a type.

`DocTypeSpec::identity` carries two shapes. `Numbered { dir }` places one `<dir>/NNNN-slug.md` file; the tool allocates the number. `Singleton { file }` places one fixed file. Both model a record that IS a single Markdown file.

A new need does not fit either shape: an **artifact** — a delivered output such as a report, a diagram, a dataset snapshot — is a document PLUS the files that document references. The document alone is not the record; the referenced files travel with it. A `Numbered` file cannot own companion files, and a `Singleton` is one fixed path.

[Issue 0018](/issues/0018-identity-cannot-express-an-author-named-record-in-a-directory-so-glossary-and-the-context-family-stay-hand-authored.md) already found that `Identity` cannot express a record living in a directory. [Issue 0024](/issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) already asked for drift detection between a document and the files it covers. An artifact is where both pressures meet, so it earns a third identity rather than a special case bolted onto a consumer.

## Decision

We will add a third `Identity` variant, `Bundle { dir }`, and register an `artifact` doc type on it.

- **Bundle shape.** An artifact record IS the directory `artifacts/NNNN-slug/`. The tool allocates the `NNNN` prefix through `next`, exactly as `Numbered` does, so an artifact is referable by number or by slug and its name cannot collide.
- **Canonical document.** The directory holds `README.md` as its one record file. `README.md` carries the OKF frontmatter and body and is subject to the canonical-frontmatter invariant. The name is `README.md`, never `index.md`: OKF reserves `index.md` for a no-frontmatter scaffold (issue 0018), and `README.md` renders when a reader opens the directory on GitHub or the web front.
- **Validated file manifest.** The frontmatter carries a `files:` list naming each referenced file, bundle-directory-relative. `check` fails when a listed file is absent and warns when the directory holds a file that the manifest does not list (an orphan). This makes doc-to-file drift a gate signal, the shape issue 0024 wants.
- **Registry row.** `ARTIFACT` joins `DOC_TYPES`: `token: "artifact"`, `identity: Bundle { dir: "artifacts" }`, `frontmatter: "Artifact"`, `index_partition: Flat`, `body_size: Exempt`, `status_vocabulary: ["Draft", "Published", "Archived"]`, `web_creatable: false`. Web authoring stays db-mode-only ([ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)); a bundle of binary files is out of that path for now, so the row opts out rather than forcing a half-built web flow.
- **Search scope.** The db-store projection and FTS index the `README.md` body, as for any record. The referenced files are opaque payload and stay out of the index.

The consumers learn the variant from the registry: `check` gains one `Bundle` arm, and a second bundle-type row later requires no edit inside `check` — the fitness shape issue 0018 named.

## Consequences

**Easier / gained:**
- Artifacts become first-class, tool-created records: `living-docs new artifact "..."` scaffolds a conformant bundle, so an artifact starts inside the frontmatter invariant and index membership rather than hand-copied outside both.
- Doc-to-file drift is caught by `check`, giving issue 0024 its first concrete instrument.
- The `Bundle` variant is reusable: any future directory-as-record type (a design package, a research bundle) rides the same identity with no new consumer code.

**Harder / accepted trade-offs:**
- `Identity` grows from two variants to three; every `match` over it gains an arm. This is the deliberate cost of expressing a shape the corpus genuinely holds.
- The manifest is authored, not derived. A referenced file added on disk but not listed only warns; the author still owns keeping `files:` truthful. Deriving the list from disk was rejected to keep the tool from guessing which files are payload versus incidental.
- Web creation of artifacts is deferred, so the web front lists but does not author them until a later decision.

**Follow-ups:**
- Slice the implementation as issues in registry → `new` → `check` order (per issue 0018): the `Bundle` variant plus the `ARTIFACT` row and `new`; then manifest validation in `check`; then the `index` Artifacts partition and the db-store projection.
- Revisit `web_creatable` once Atlas has an upload path for bundle payload files.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs` (the `Identity` enum and the `ARTIFACT` row), `living-docs-core/src/paths.rs` (`dir_for`/`doc_type_for_dir` over the new variant), `living-docs-core/src/commands/new.rs` (scaffold a bundle directory with `README.md`), `living-docs-core/src/commands/check.rs` (manifest validation) and `index.rs` (Artifacts partition), plus `skills/living-docs/templates/artifact.md`.

**Verification criteria:**
- `living-docs new artifact "..."` produces `artifacts/NNNN-slug/README.md` that `living-docs check docs` passes without hand-editing.
- Fitness function: a bundle record whose `files:` names an absent file fails `check`, and `check` learns the `Bundle` variant from the registry — adding a second `Bundle` row requires no edit inside `check`.
