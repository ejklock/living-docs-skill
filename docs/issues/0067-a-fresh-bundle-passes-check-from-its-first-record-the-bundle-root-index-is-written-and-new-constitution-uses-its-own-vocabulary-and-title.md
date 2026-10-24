---
type: Issue
title: "A fresh bundle passes check from its first record: the bundle-root index is written and new constitution uses its own vocabulary and title"
description: new and index write a missing bundle-root index.md so a new bundle is conformant, and new constitution writes a status from the constitution vocabulary and the given title as its heading
status: open
timestamp: 2026-10-24T10:17:30Z
---

## 0067. A fresh bundle passes check from its first record: the bundle-root index is written and new constitution uses its own vocabulary and title

Found by the usage audit and memory lesson 1537 (living-docs 0.23.0, verified again on 2026-10-24): in an empty repository, `new adr` followed by `check` fails with "missing bundle-root index.md (invariant 3)" until `docs/index.md` is written by hand; `new constitution` writes `status: Proposed`, outside the constitution vocabulary, and a fixed heading that `check --advisories` flags as HEADING.

### Scope

- `new` and `index` write `<docs-dir>/index.md` when it is missing, with the frontmatter and the links to each type index that `check` requires; an existing bundle-root index is never rewritten.
- `new constitution` writes the first status of the constitution vocabulary and the given title as its heading.

### Decision

- The bundle-root index is created once and then left to the author; option not taken: regenerating it on every run, which would drop hand framing.
- When a present root index does not reach a type index that `new` or `index` just wrote, one link line for it is appended at the end of the file and every existing byte stays; option not taken: never touching a present root, which makes the bundle fail again the first time a second record type arrives.
- A type with an empty status vocabulary keeps its template's own `status:` literal, so the constitution seeds `Draft`; option not taken: registering the constitution's vocabulary in the doc-type table, which would change what `check` and `set` accept on downstream constitutions.
- The constitution template's heading becomes a placeholder that `new` fills with the given title, unnumbered.

### Acceptance

- In an empty git repository, `new adr "A"` then `check` reports no invariant-3 violation for the bundle root (unfilled slots aside).
- A present `docs/index.md` is byte-identical after `new` and `index`.
- `new constitution "Our rules"` writes a status of the constitution vocabulary and a heading carrying "Our rules"; `check --advisories` reports no HEADING advisory for it.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: bundle-root index creation shared by `new` and `index`; tests red first.
2. Core: constitution status and heading.
