---
type: Issue
title: new, set and supersede regenerate the touched index in the same run
description: "Implements the ADR that makes the mutation verbs leave the bundle indexed: each of new, set and supersede rebuilds the index of the doc types it touched, reports it, and index stays as the repair verb."
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0055. new, set and supersede regenerate the touched index in the same run

Implements [ADR 0065](/adr/0065-mutation-verbs-leave-the-bundle-indexed-new-set-and-supersede-regenerate-the-touched-type-index-in-the-same-run.md). Each mutation verb regenerates the index of the doc types it touched, so a CLI-authored record is never an orphan and `index` becomes the repair verb.

### Scope

- `new` regenerates the created type's index; `set` the record's type, after a retitle too; `supersede` the old and the new record's types.
- The verbs reuse the `index` computation and write only when the content differs.
- JSON output of `new`, `set` and `supersede` gains `indexed` (the index paths written, possibly empty). Text output keeps its stdout lines and prints one informational stderr line per written index, silenced by `--quiet`.
- A failed index write after a successful record write exits 1 and names the record and `living-docs index <type>`.
- `index` itself is unchanged. The constitution regenerates nothing.
- The procedure topic, the issue-workflow topic, the session hook text, the body-only instruction and the README stop telling the author to run `index` after a verb and name it as the repair verb.

### Decision

- The per-type regenerate lives in the `index` module as one public entry point the three verbs call; option not taken: each verb recomputing rows on its own.
- The stderr line reads `living-docs <verb>: indexed <path>`; option not taken: adding the path to stdout, which would change the first-line contract consumers parse.

### Acceptance

- `new issue "T"` followed only by `check` passes, and the issue index lists the record.
- `set NNNN status closed` leaves the index row showing `closed`; `set NNNN title "New"` leaves the row pointing at the renamed file.
- `supersede OLD NEW` leaves both rows carrying the successor; when OLD and NEW are of different types both indexes are regenerated.
- `index` after any of the above writes nothing (`git status` clean on the index files).
- With `--json`, each verb's output carries `indexed`; with `--quiet`, no index line reaches stderr.
- An unwritable index file makes the verb exit 1 with the record written and the repair command in the message.
- The full suite, `make check` and `living-docs check` over `docs/` pass.

### Plan

1. Core: a per-type regenerate entry point in `index.rs`; `new`, `set`, `supersede` call it and return the written paths; unit tests red first.
2. CLI: `indexed` in the three JSON shapes, the stderr line, the exit-1 path; integration tests through the binary.
3. Docs: procedure, issue-workflow, session hook, README, body-only instruction.

### Outcome

Delivered, uncommitted, in the working tree: `new`, `set` and `supersede` refresh the index of every type they touch before returning (`living-docs-core/src/commands/index/refresh.rs`), write it only when the content differs, report each written index on stderr (`living-docs <verb>: indexed <path>`, silenced by `--quiet`) and in the JSON field `indexed`; a record written whose index could not be refreshed exits 1 with `run living-docs index <type>` as the hint. `index` stays as the repair verb and the body-only instruction, the skill topics, the session hook and the README say so. Proof: `cargo test -p living-docs --test mutation_verbs_index` (12 passed), `cargo test --workspace` green (696), `se-gates check` pass after replacing the two pre-existing `expect()` in the touched core files.
