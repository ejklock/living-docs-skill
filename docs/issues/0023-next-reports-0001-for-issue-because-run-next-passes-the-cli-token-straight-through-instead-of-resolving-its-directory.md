---
type: Issue
title: next reports 0001 for issue because run_next passes the CLI token straight through instead of resolving its directory
description: Fix run_next to resolve the CLI token to its record directory via paths::dir_for before scanning, the way new already does.
status: closed
timestamp: 2026-08-03T12:09:03Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## next reports 0001 for issue because run_next passes the CLI token straight through instead of resolving its directory

Found while standardizing a batch of issue records in this same session.

### Problem

`living-docs next issue` reports `0001` in a bundle that already holds 22 issues, while
`living-docs next adr` correctly reports the next free number. Root cause:
`commands::next::next_number`/`next_number_from_store` (`living-docs-core/src/commands/next.rs`)
are documented and tested to expect the record's **directory** name (e.g. `issues`), not the
CLI token (`issue`). `run_next` in `cli/src/main.rs` calls `commands::next::run(docs_dir,
doc_type)` with the raw token, unresolved:

```rust
fn run_next(docs_dir: &Path, doc_type: &str) -> ExitCode {
    commands::next::run(docs_dir, doc_type)
}
```

`issue` is the only registered doc type whose token (`issue`) differs from its directory
(`issues`, per `paths::dir_for` and `DocTypeSpec::Identity::Numbered { dir: "issues" }`) — so
`adr`/`bdr`/`prd` happen to work by coincidence (token equals directory), and `issue` is the
one case that exposes the missing translation. `commands::new::plan_at` does not have this
bug: it already resolves the token to `spec.directory_name` before calling
`next_number_from_store`, which is why `new issue` numbers correctly while `next issue` does
not.

### Scope

Included: `run_next` resolves `doc_type` through `paths::dir_for` before calling
`commands::next::run`, the same translation `new` already performs.

Explicitly out: any change to `new`'s numbering (already correct), or to `next.rs`'s own
scan logic (already correct once given the right directory name).

### Acceptance

- `living-docs next issue` reports one past the highest existing issue number.
- A regression test pins this against a fixture bundle holding issue records, so a future
  doc type whose token and directory diverge cannot reintroduce it silently.
- An unknown doc-type token still fails cleanly (no behavior change for the existing error
  path).

### Plan

Change `run_next` to look up `paths::dir_for(doc_type)` and pass the resolved directory (or
a clean CLI error for an unknown token) to `commands::next::run`, mirroring how `new`
resolves the same token today.
