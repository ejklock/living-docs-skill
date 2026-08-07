---
type: Issue
title: Bundle identity variant plus the artifact registry row and new artifact scaffolds a directory bundle with README
description: Add the Bundle identity variant and the artifact registry row, and make new artifact scaffold a directory bundle with a canonical README.
status: open
timestamp: 2026-08-07T14:38:11Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Bundle identity variant plus the artifact registry row and new

Implements [ADR 0034](/adr/0034-artifacts-are-directory-bundle-records-a-canonical-readme-plus-a-validated-file-manifest-on-a-new-bundle-identity.md). First slice: teach the domain and `new` about a directory-bundle record, so an artifact can be created at all.

`Identity` has two variants, both modeling a record that is a single file. This slice adds `Bundle { dir }` for a record that IS a directory holding a canonical `README.md`, and registers the `artifact` doc type on it.

### Scope

Included: the `Bundle { dir }` variant on `Identity`; the `ARTIFACT` row in `DOC_TYPES` (`token: "artifact"`, `identity: Bundle { dir: "artifacts" }`, `frontmatter: "Artifact"`, `index_partition: Flat`, `body_size: Exempt`, `status_vocabulary: ["Draft", "Published", "Archived"]`, `web_creatable: false`); `paths::dir_for`/`doc_type_for_dir` honoring the variant; the `artifact.md` template; and `living-docs new artifact "..."` scaffolding `artifacts/NNNN-slug/README.md` with the `NNNN` allocated by the existing `next` counter.

Explicitly kept: `Numbered` and `Singleton` keep their meaning and rows. This slice adds a variant and one row; it reshapes nothing.

Explicitly out: manifest validation (issue 0032) and the index/search wiring (issue 0033). `new` seeds an empty `files:` list; it does not validate it.

### Acceptance

- `living-docs new artifact "A title"` creates `artifacts/0001-a-title/README.md` carrying canonical `Artifact` frontmatter with an empty `files:` list, and `living-docs check docs` passes it without hand-editing.
- A second `new artifact` allocates `0002-...`, proving the `next` counter drives the bundle number.
- Fitness function: `spec_for("artifact")` round-trips, and the template's `type:` line equals the row's `frontmatter` value (the existing `fitness_function_a` covers the new row with no edit).

### Plan

Registry first: add the `Bundle` variant and the `ARTIFACT` row, then the `artifact.md` template, then the `new` scaffold path that creates the directory and writes `README.md`. Pin the number allocation and the frontmatter shape with tests.
