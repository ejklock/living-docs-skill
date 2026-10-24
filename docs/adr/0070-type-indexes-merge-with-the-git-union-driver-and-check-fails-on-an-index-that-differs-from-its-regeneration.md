---
type: ADR
title: Type indexes merge with the git union driver, and check fails on an index that differs from its regeneration
description: install hooks adds a merge=union .gitattributes line for the bundle's index.md files so parallel branches stop conflicting on derived listings; check regenerates each type index in memory and fails on drift, naming living-docs index as the repair
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-24T10:15:27Z
---

# 0070. Type indexes merge with the git union driver, and check fails on an index that differs from its regeneration

## Context

Every `new` on a branch appends a row to the same place in its type's `index.md`. Two branches that each add a record therefore conflict in the index on every merge, although the index is derived data that `living-docs index` regenerates ([ADR 0065](/adr/0065-mutation-verbs-leave-the-bundle-indexed-new-set-and-supersede-regenerate-the-touched-type-index-in-the-same-run.md)). A usage audit over the owner's agent transcripts of four harness accounts (October 2026) found 349 mentions of conflicts in `docs/issues/index.md` across 75 sessions of one account alone, and 176 shell edits of an `index.md`: agents resolve the conflict by hand instead of regenerating.

`check` does not see a drifted index. It reports an unlisted record (orphan) and a row whose link is broken, but a duplicated row, a row with a stale status, or rows out of order pass. A hand resolution or a line-based merge can therefore leave a wrong index that the gate accepts.

Git ships a built-in `union` merge driver that keeps the lines of both sides instead of writing conflict markers. It is configured through a committed `.gitattributes` line, so it travels with every clone.

## Decision

We will merge type indexes with git's `union` driver and make `check` fail on an index that differs from its regeneration.

- **`.gitattributes`.** `living-docs install hooks` adds `<docs-dir>/**/index.md merge=union` to the project's `.gitattributes`, creating the file when absent and keeping every other line; running it again adds nothing. `living-docs uninstall hooks` removes that line only.
- **Index drift is a violation.** `check` regenerates each type index in memory with the computation `index` uses and fails when the file differs, naming the index and the repair `living-docs index <type>`. A bundle the CLI wrote never trips it, because `index` is idempotent ([ADR 0001](/adr/0001-living-docs-cli.md)).
- **The merge flow.** A merge of two branches that both added records completes without a conflict in the index; the merged index may hold rows out of order or twice; the pre-commit gate fails on it; one `living-docs index` repairs it.
- **`check` stays read-only** ([ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md)): it reports the drift, it never writes the index.

Rejected alternatives:

- **A custom merge driver that runs `living-docs index`.** No manual step, but the driver is defined in `.git/config`, which is not versioned: every clone must run `install` before its first merge, and git would spawn the binary inside a merge.
- **Keep the conflict and teach `index` as the repair.** That is the current rule, and the transcripts show it does not hold.
- **Stop committing indexes.** A bundle browsed on a forge or read without the binary would have no listing, against the OKF index invariant.

## Consequences

**Easier / gained:**
- Parallel branches stop conflicting on the derived file, which is the most frequent conflict in the transcripts.
- A wrong index, from a merge or a hand edit, fails the gate instead of shipping.

**Harder / accepted trade-offs:**
- A bundle whose index drifted fails `check` after the upgrade; `living-docs index` is the one-command repair.
- `union` is line-based. Prose that both sides changed in an index preamble is kept twice; regeneration keeps the preamble as written, so that duplication survives and is visible only in the diff.
- A project that never runs `install hooks` keeps the conflict; the drift violation still catches a bad hand resolution.

**Follow-ups:**
- The procedure topic and the README describe the merge flow and the repair.

## Verification

**Implementation impact:** `cli/src/hooks.rs`, `cli/src/commands/install.rs`, `cli/src/commands/uninstall.rs`, `living-docs-core/src/commands/index.rs` (the in-memory generation, shared), a check module for index drift, `living-docs-core/src/check/kinds.rs`, `skills/living-docs/rules/procedure.md`, `README.md`, `cli/tests/*`.

**Verification criteria:**
- `install hooks` run twice leaves exactly one `merge=union` line for the bundle's indexes and keeps a pre-existing foreign line; `uninstall hooks` removes only that line.
- In a real git repository with the attribute, two branches that each run `new issue` merge without a conflict in `docs/issues/index.md`; `check` then fails naming that index, and after `living-docs index` it passes.
- An index with a hand-edited status or a duplicated row makes `check` exit 1 naming the index; a freshly indexed bundle reports no drift.
