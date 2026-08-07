---
type: Issue
title: index renders the Artifacts partition and the db-store projection indexes the artifact README body
description: Render an Artifacts partition in index and index each artifact README body in the db-store projection and FTS.
status: open
timestamp: 2026-08-07T14:38:11Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## index renders the Artifacts partition and search indexes the README

Implements [ADR 0034](/adr/0034-artifacts-are-directory-bundle-records-a-canonical-readme-plus-a-validated-file-manifest-on-a-new-bundle-identity.md). Third slice: make artifacts discoverable — listed in the index and findable in search.

After issues 0031–0032 an artifact bundle is created and validated, but it appears in no index and no search result. This slice wires the two read paths.

### Scope

Included: `index` renders a flat `Artifacts` partition listing every `artifacts/NNNN-slug/` bundle by its `README.md` title and status; the db-store projection and FTS index the `README.md` body of each artifact, keyed like any other record. Both derive membership from the `ARTIFACT` row, not a hand-coded path.

Explicitly kept: the existing index partitions and their ordering; the search behavior for all current types.

Explicitly out: indexing the referenced payload files — they stay opaque per ADR 0034; only the `README.md` body is searchable.

### Acceptance

- `living-docs index` writes an `Artifacts` section listing each bundle, and the run is idempotent (a second `index` leaves the file byte-identical).
- After a sync, `living-docs search "<term from a README body>"` returns the artifact record; searching a term that appears only in a payload file returns nothing.
- `living-docs check docs` stays green with artifact bundles present in the tree.

### Plan

Add the `Artifacts` partition to `index` off the registry row, then extend the db-store projection to read the bundle `README.md`. Cover index idempotence and the README-only search scope with tests.
