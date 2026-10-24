---
type: ADR
title: Duplicate record numbers fail check, and set number renumbers a record with its links and index
description: Two records of one type with one number become a check violation naming both paths; set <ref> number <NNNN|next> renames the file, heading and every in-bundle link and regenerates the index, on any status; the surface stays ten verbs
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-24T10:15:26Z
---

# 0069. Duplicate record numbers fail check, and set number renumbers a record with its links and index

## Context

Parallel branches each run `living-docs new`, and each takes the next free number of its own tree. When they merge, the bundle holds two records with one number. Nothing stops it: `check` passes a bundle where `docs/issues/0055-a.md` and `docs/issues/0055-b.md` both exist and are both indexed, and every reference by number to either of them becomes ambiguous.

This repository carries seven such pairs today, issues 0055 to 0061 (`ls docs/issues | grep -oE '^[0-9]{4}' | sort | uniq -d`): six were filed on main while a feature branch had already taken the same numbers. A usage audit over the owner's agent transcripts of four harness accounts (October 2026) found 349 mentions of merge conflicts in `docs/issues/index.md` and fleet managers coordinating numbers by message ("message the manager BEFORE running living-docs new, so the numbers do not collide"). The repair today is by hand: `git mv`, a `sed` over every link to the old path, then `index`.

[ADR 0061](/adr/0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md) already gives `set title` the machinery a renumbering needs: rename the file, rewrite the heading, rewrite every in-bundle link, regenerate the index.

## Decision

We will make a duplicate number a `check` violation and give `set` a `number` key that renumbers one record.

- **The violation.** `check` fails when two records of the same numbered type share a number, whatever their status. The message names both paths and the repair, `living-docs set <path> number next`; the path form of the reference is the one that stays unambiguous while the duplicate exists.
- **`set <ref> number <NNNN|next>`.** Renames the file to the new number with its slug unchanged, rewrites the number in the record's heading, rewrites every in-bundle link to the old path, and regenerates the type index ([ADR 0065](/adr/0065-mutation-verbs-leave-the-bundle-indexed-new-set-and-supersede-regenerate-the-touched-type-index-in-the-same-run.md)). `next` takes the next free number of the type. A number already taken, a number of the wrong width, or a type without numbers (architecture view, constitution) exits 2 and writes nothing.
- **Every status.** Unlike a retitle, a renumbering is allowed on a closed or superseded record: two closed records can collide, and without the key there would be no repair.
- **The author chooses which record moves.** The tool reports both and renumbers the one it is told to; it never guesses which one is canonical.
- **Ten verbs.** `number` is a key of `set`, so the surface [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) fixed stays at ten verbs. `set` already performs a rename for `title`.

Rejected alternatives:

- **A `renumber` verb.** The name is more explicit, but it grows the ten-verb contract for a behavior `set` already has the machinery for, and every document naming the verbs would change.
- **An advisory instead of a violation.** Duplicates would keep merging silently; an advisory does not fail the pre-commit gate that a parallel merge passes through.
- **Reserving numbers ahead of a merge.** Branches share no state, and a reservation file would itself conflict at merge. The tool stays deterministic over its inputs: the tree it runs in.

## Consequences

**Easier / gained:**
- A collision fails at the first commit after the merge, with the one-command repair in the message.
- A renumbering never leaves a broken link or a stale index.

**Harder / accepted trade-offs:**
- A bundle that already holds duplicates fails `check` after the upgrade. This repository renumbers its seven open duplicates in the change that ships the violation.
- References outside the bundle (commit messages, an external tracker) still name the old number; the tool does not rewrite them.

**Follow-ups:**
- The issue that implements this record also renumbers this repository's open duplicates.
- The procedure topic, the issue-workflow topic and `set --help` name the key and the repair.

## Verification

**Implementation impact:** `living-docs-core/src/check/records.rs` or a sibling check module, `living-docs-core/src/check/kinds.rs`, `living-docs-core/src/commands/set.rs`, `living-docs-core/src/commands/set/retitle.rs` (shared rename and link rewrite), `living-docs-core/src/commands/next.rs`, `cli/src/commands/set.rs`, `skills/living-docs/rules/procedure.md`, `skills/living-docs/rules/issue-workflow.md`, `README.md`, `cli/tests/*`.

**Verification criteria:**
- Two indexed records `issues/0007-a.md` and `issues/0007-b.md` make `check` exit 1 with one violation naming both paths; one record per number passes.
- `living-docs set docs/issues/0007-b.md number next` renames the file to the next free number, rewrites its heading and every in-bundle link to it, and `check` then passes with no `index` call.
- `set <ref> number` with a taken number, a malformed number, or on an architecture view exits 2 and leaves the tree unchanged.
- `living-docs index` after a renumbering writes nothing.
