# Semantic Index Organization

Documentation is organized by **meaning, not by accident of growth**. When a body of knowledge is large, it is split into semantically coherent files, each with a single concern, all reachable from one index. The index is the entry point; the group files are the content.

This applies to any large doc — most commonly the domain/module **context** (vocabulary), but equally to research, ADR collections, or any directory that accumulates files.

## The indexing contract

1. **Every directory has an `index.md`** — the OKF reserved directory listing (§6), for collections (ADRs/issues/research) and split single docs (context) alike. It lists every file with a one-line description and carries **no frontmatter** (the sole exception: the bundle-root `docs/index.md`, which may declare `okf_version: "0.1"`).
2. **Every file is reachable from an index, and from the bundle-root `docs/index.md`** linked by the project guide. No orphans.
3. **One concept, one file.** Each group file owns a coherent slice of the vocabulary or content. A concept appears in exactly one group file; other files cross-reference it. Every group/concept file opens with OKF frontmatter carrying a non-empty `type`.
4. **The index carries no content** — only intro framing + a table/list of pointers. Content lives in the group files, never duplicated into the index.
5. **Links resolve.** Every pointer in an index points to a file that exists; prefer bundle-relative (`/…`) links. Check after every split or move.

## When and how to split a large doc

Split when a doc passes ~200 lines or starts mixing unrelated concerns.

1. **Map sections → groups.** Lay out a source map: each section of the old file → exactly one new group file. Decide group boundaries by *meaning* (write-path vs read-path, domain concepts vs storage shapes), not by length.
2. **Content-preserving move.** Move vocabulary verbatim. Do not reword, add, or drop terms during a split — that conflates two changes and makes the diff unreviewable. Reword in a *separate* later change if needed.
3. **One concept lands once.** If two sections discuss the same concept, pick its home and cross-reference from the other.
4. **Build the index** — intro paragraph + a TOC table linking every group file with a one-line description. See `templates/context-index.md`.
5. **Cut over.** Repoint the live pointers (project guide's Docs index, maintenance rules) to the new index. Delete the old monolith. Leave *historical* mentions (in ADR/issue "Consequences") untouched — they are history.
6. **Completeness review.** Diff the old content against the union of new files: every term present exactly once, nothing lost or duplicated, all index links resolve, old file removed.

## Heading discipline

Each group file is a standalone document: it leads with a single `#` H1 title, then `##` sections. Do not carry over `##`-as-top-level headings from the section you extracted — promote them to H1 so the file reads as its own document.

## Anti-patterns

- An index that duplicates content from the group files — now there are two homes and they will drift.
- Splitting by line count into arbitrary "part 1 / part 2" files instead of by meaning.
- Rewording vocabulary during a split — bundles two changes, breaks the content-preserving guarantee.
- A group file with two unrelated concerns because "it was already there."
