---
type: Issue
title: "A fresh bundle passes check from its first record: the bundle-root index is written and new constitution uses its own vocabulary and title"
description: new and index write a missing bundle-root index.md so a new bundle is conformant, and new constitution writes a status from the constitution vocabulary and the given title as its heading
status: closed
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

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog.

- `living-docs-core/src/commands/index/root.rs` (new): `ensure_root_index` creates a missing bundle-root `index.md` once (okf frontmatter plus one link line per type index present) and, on a present root, appends one link line per type index the root does not already reach, judged by the same reachability `check` uses. Existing bytes are never rewritten; the appended line follows the root's own line ending (CRLF when the file holds one). A root that is not a regular file or cannot be read is an error naming it; nothing on disk is removed.
- `new` and `index` call it. `new` on a singleton (the constitution) writes a missing root with no links, so a bundle started with `new constitution` passes `check` at once; `index` alone writes a root only when a type index exists. `new --json` keeps its contract: the root is not listed in `indexed`.
- A root that cannot be written after the record and its type index are written exits 1 with its own repair line: "the bundle-root index could not be written (...) — fix that path or its directory, then run `living-docs index`". A type-index failure keeps its existing line.
- `new` keeps a template's own `status:` literal for a type with an empty vocabulary, so the constitution seeds `Draft`; its template heading is a placeholder filled with the given title, unnumbered.
- Duplication removed while touching the files: `index.rs` reuses `next.rs`'s filename-number parser, and `new/fill.rs` shares one frontmatter line-mapping helper. `check/mod.rs` re-exports only the two graph functions the root writer needs and stays at 300 lines. Record citations were removed from the docblocks of the touched files.
- Docs: README.md, `skills/living-docs/rules/adr-conventions.md` and `constitution-conventions.md` describe the behavior.
- Tests: `cli/tests/fresh_bundle.rs` (16, in git repositories) and `index/root_tests.rs` (9). Reviewers planted faults for each behavior (always append, no separator, root error swallowed, empty vocabulary back to Proposed, singleton root skipped, old repair line, CRLF ignored); each failed a test.
- Verification in the main checkout: `cargo test --workspace` 880 passed, 0 failed; `make check` (file-size ratchet and allow-inventory included), clippy with `-D warnings`, `cargo fmt --check`, `living-docs index` (no-op on docs/), `living-docs check docs` and `se-gates check` pass. Review approved round 1 with three normal findings and five nits; all fixed; one fix pushed check/mod.rs to 301 lines, caught and fixed in round 3; final narrow re-review approved.
