---
type: Issue
title: Implement union merge of type indexes and the index-drift violation
description: "Implements ADR 0070: install hooks writes a merge=union .gitattributes line for the bundle's index.md files, uninstall removes it, and check fails on an index that differs from its in-memory regeneration"
status: open
timestamp: 2026-10-24T10:17:27Z
---

## 0064. Implement union merge of type indexes and the index-drift violation

Implements [ADR 0070](/adr/0070-type-indexes-merge-with-the-git-union-driver-and-check-fails-on-an-index-that-differs-from-its-regeneration.md).

### Scope

- `living-docs install hooks` adds `<docs-dir>/**/index.md merge=union` to the project's `.gitattributes`, idempotently and keeping foreign lines; `uninstall hooks` removes that line only.
- `check` regenerates each type index in memory with the computation `index` uses and reports a violation when the file differs, naming the index and `living-docs index <type>`.
- The procedure topic and the README describe the merge flow and its repair.

### Decision

- The in-memory generation is the one `index` writes, exposed without a write; option not taken: a second comparison that parses rows.
- A preamble kept by `index` is part of the comparison as written, so only the generated listing can drift.

### Acceptance

- `install hooks` twice leaves one attribute line and a pre-existing foreign line intact; `uninstall hooks` removes only the attribute line.
- A test in a real git repository: two branches each run `new issue` and merge without a conflict in `docs/issues/index.md`; `check` then fails naming that index; after `living-docs index` it passes.
- A hand-edited status or a duplicated row in an index makes `check` exit 1; a freshly indexed bundle and this repository report no drift.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: in-memory index generation shared with `index`; the drift check and its kind; tests red first.
2. CLI: the attribute line in install and uninstall; the git merge integration test.
3. Topics and README.
