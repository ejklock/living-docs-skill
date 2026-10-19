---
type: ADR
title: "Mutation verbs leave the bundle indexed: new, set and supersede regenerate the touched type index in the same run"
description: new, set and supersede regenerate the index of every doc type they touched before returning, so index becomes a repair verb and a model never pays a separate call or a check round for an index row.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-19T13:13:52Z
---

# 0065. Mutation verbs leave the bundle indexed: new, set and supersede regenerate the touched type index in the same run

## Context

Every mutation verb leaves the bundle one step short of conformant. `new` writes the record but not its index row; `set status` changes the record but the index still shows the old status; `supersede` wires both records and leaves two stale rows. The author must remember `living-docs index`, and `check` fails with "not listed in index.md — orphan" when they do not.

A usage analysis over 8,376 CLI calls recorded in the owner's agent transcripts (September and October 2026, four harness accounts) shows what that costs. `index` is 953 of those calls, 477 of them a standalone call issued only to repair what the previous verb left; the orphan violation appears 18 times; 188 times an agent edited an `index.md` by hand instead; and 33 sessions ran `check` sixteen or more times, in the literal sequence `set`, `index`, `check`, `set`, `index`, `check`. The procedure topic has told agents to run `index` since the first release; the data says the instruction does not hold.

[ADR 0048](/adr/0048-cli-mutation-verbs-emit-canonical-frontmatter-so-check-never-routes-a-user-to-fmt.md) settled the same question for frontmatter: a verb the CLI owns leaves its own output canonical, so `check` never routes the author to `fmt`. The index is the last CLI-owned artifact a verb leaves stale. Regenerating one type's index over this repository's 110 records takes 50 ms (`/usr/bin/time living-docs index`).

## Decision

We will make `new`, `set` and `supersede` regenerate the index of every doc type they touched before they return, so the bundle they leave passes the index invariants without a further call.

- **Which indexes.** `new` regenerates the index of the created type; `set` the index of the record's type, including after a retitle that renamed the file; `supersede` the index of the old record's type and of the new one's when they differ. A singleton type with no directory index (the constitution) regenerates nothing.
- **Same computation as `index`.** The verbs call the computation the `index` verb uses and write only when the generated content differs from the file, so `index` after any of them is a no-op and the idempotence fitness function of [ADR 0001](/adr/0001-living-docs-cli.md) holds.
- **Reporting.** The JSON success output of each verb gains an `indexed` field listing the index paths written (a field is added, never renamed, per [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md)). Text output keeps its stdout lines and reports each written index on stderr as an informational line that `--quiet` silences.
- **Failure.** When the record write succeeds and the index write fails, the verb exits 1, names the written record and the repair command `living-docs index <type>`, and leaves the record in place: the author's work is never rolled back for a derived file.
- **`index` stays.** It becomes the repair verb for a brownfield bundle, a hand-edited tree or a merge, and keeps its contract unchanged.

Rejected alternatives:

- **Keep `index` separate and teach harder.** The procedure topic, the session hook and the body-only instruction have named the step for every release; 477 standalone repair calls and 188 hand edits of an index say teaching does not close the gap.
- **`check --fix` that writes the index.** It makes the gate a writer. [ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) removed the write-time hook so that `check` is the only enforcement and never mutates the tree.
- **An `--index` opt-in flag on each verb.** The default is what agents reach for; an opt-in flag is one more thing to know and would be as unused as `index` is remembered.
- **A `--no-index` opt-out.** No caller asked for a stale index; a bundle that keeps hand-written text below an index table already loses it on `index` itself, which is that verb's own defect.

## Consequences

**Easier / gained:**
- A record authored through the CLI is never an orphan; the orphan violation class disappears for CLI-authored records and one call per mutation leaves the agent's loop.
- `index` has one meaning: repair.

**Harder / accepted trade-offs:**
- Every mutation verb now reads the type's directory and may write a second file. At 50 ms per 110 records the cost is below the latency of the call itself.
- A bundle that keeps hand-written sections below an index table loses them on the first mutation verb, as it already does on `index`. That behaviour of `index` is a separate defect to track.

**Follow-ups:**
- [Issue 0055](/issues/0055-new-set-and-supersede-regenerate-the-touched-index-in-the-same-run.md) implements the three verbs, the `indexed` field and the repair message.
- The procedure topic, the issue-workflow topic, the session hook text and the README drop the "then run `index`" step and name `index` as the repair verb.

## Verification

**Implementation impact:** `living-docs-core/src/commands/new.rs`, `set.rs`, `supersede/mod.rs`, `index.rs` (a per-type regenerate entry point), `cli/src/commands/new.rs`, `set.rs`, `supersede.rs`, `skills/living-docs/rules/procedure.md`, `issue-workflow.md`, `skills/living-docs/hooks/session-context.sh`, `README.md`.

**Verification criteria:**
- After `living-docs new issue "T"` with no further call, `living-docs check` passes and `docs/issues/index.md` lists the record.
- After `living-docs set NNNN status closed`, the index row shows `closed`; after `living-docs supersede OLD NEW`, both rows carry the successor.
- `living-docs index` after any of the three verbs writes nothing.
- An integration test in `cli/tests` runs `new` then `check` with no `index` call and fails the build when the record is an orphan.
