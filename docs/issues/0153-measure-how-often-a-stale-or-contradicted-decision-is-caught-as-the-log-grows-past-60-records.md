---
type: Issue
title: Measure how often a stale or contradicted decision is caught as the log grows past 60 records
description: A seeded eval plants contradictions in 60- and 240-ADR bundles and measures how often agents catch them, before building candidates at write new or an amend relation.
status: open
timestamp: 2026-11-23T12:14:02Z
---

## 0153. Measure how often a stale or contradicted decision is caught as the log grows past 60 records

An outside comment, 2026-11-23: "the model noticing a contradiction works until it doesn't, and then nobody knows a decision went stale; how does it hold up once the log gets past 60 or so?" This bundle holds 103 ADRs, 59 in force. A design review the same day found that no deterministic signal catches a contradiction between two decisions in force, and that every stale decision of that session was caught by a person or an agent reading, never by the tool.

### Scope

- Measure, with the existing eval harness (`evals/`), how often an agent catches a contradiction it should resolve when the bundle holds 60 and 240 ADRs, before building anything.
- After the measurement, and only if its decision rule says so: candidate records printed by `write new adr` (O2), and a declared amend relation (O1).

Kept: `write supersede` and its chain checks; the determinism boundary (no model inside the binary).

### Decision

- Failure taxonomy (design review, 2026-11-23): T1 contradicted by a newer decision with no supersede; T2 partially reversed with no supersede (supersede retires a whole record, so a partial reversal is written as prose, as ADR 0099 did against ADR 0027); T3 contradicted by code; T4 never implemented; T5 premise invalidated by the world. Today `check` catches part of T3 and T4 (`path-missing`, `stale-view`) and none of T1, T2 or T5. Supersede closes a declared, whole replacement only.
- Measure first (owner, 2026-11-23): the scale eval (O4) runs before any build; then candidates at `write new adr` (O2) and a declared amend relation (O1) as the measurement decides. Not taken: building O2 and O1 at once; adding the `verification-unbound` advisory (O3) now; a scope-overlap advisory (O6, the DUPLICATE pass ADR 0057 cut); a `reviewed:` date on every record (O7).
- Decision rule, fixed before running: if the unprompted catch rate at 240 ADRs is below the prompted arm's, or below the unprompted rate at 60 ADRs, build O2 and rerun the same scenarios against it; then decide O1 from the T2 numbers.
- After the measurement (owner, 2026-11-23): the rule says build O2, so an advisor designs how `write new adr` finds candidates deterministically before an ADR; O1 waits. Not taken: deciding O1 now; stopping at the measurement.
- O2 is measured in variants, smallest first (owner, 2026-11-23, after a design review): the transcripts show unprompted sessions already ran `read records --topic`, got an empty answer on a paraphrased conflict and read it as no conflict. V1 rewords the empty `--topic` answer (records in force, no word match is not evidence of no conflict, `read records` lists them); V2 adds one fixed line to `write new adr` asking whether the decision contradicts or partly reverses a record in force; V3 (candidates by rare shared terms) is built only if V1 and V2 fall short. Rule fixed before running: keep the smallest variant whose Sonnet unprompted paraphrased catch reaches 11/12 with no false alarm and no over-link. The scorer first separates superseded from merely named and counts a non-plant link as over-linked, and noise controls are added. Not taken: V1 alone; V3 directly (candidates on 84 to 94 of 94 writes over this repository's ADRs); pausing O2.

### Acceptance

- Ten scenarios (T1, T2 and a control with no plant; 60 and 240 ADRs; the conflicting record worded with the task's words and paraphrased), two arms (unprompted task; task that asks for conflicting record numbers in `CONFLICTS.txt`), two models, three runs each, scored by `expect.sh`: caught means the planted record is superseded by or linked from the new record (unprompted) or listed (prompted); the control fails on any supersede or listed number.
- An aggregate over the runs' `score.json`: catch and false-alarm rate per type, size, wording, arm and model, plus `read_before_first_write`, recorded in this issue's Outcome with the cost of the runs.
- The scenario generator is seeded and deterministic; the binary is not changed.

### Plan

- S1: the scale eval — seeded generator, T1/T2/control scenarios, prompted and unprompted arms, scorer, aggregator, and one recorded run | files: evals/ | after: none
- S2: scorer separates superseded from named-only and counts over-links; noise-control scenarios whose task shares rare words with a non-conflicting record; raw run outputs kept outside any worktree | files: evals/scale, evals/tests | after: S1
- S3: V1, the empty `read records --topic` answer names the records in force, says no word match is not evidence of no conflict, and points at `read records` | files: living-docs-core/src/commands/effective, cli/tests | after: S1
- S4: the ADR for V2 and V2 itself, one fixed ask line and `in_force` on `write new adr` and the MCP `write` tool | files: docs/adr, living-docs-core/src/commands/new, cli, mcp | after: S3
- S5: measure V1 and V2 (Sonnet, unprompted, 30 runs each) against the fixed rule, then the chosen variant over the full 120 runs, and record the numbers | files: docs/issues | after: S2, S4
- S6: decide O1 (amend relation) from the T2 numbers, by ADR | files: docs/adr | after: S5

### Outcome

- S1 built the scale eval under `evals/scale/` (commits `3418fbf`, `a83e03e`, `4d26218`): a seeded generator (seed 153) writes ten scenarios in two arms, `expect.sh` scores catch, miss and false alarm, and `aggregate.sh` reports rates and the decision rule. `evals/tests/run.sh` holds 239 cases, all green.
- Full run, 2026-11-23: 3 fresh repetitions, models one at a time, 120 runs, isolated runner `evals/scale/runner.sh`. Cost USD 7.58: Sonnet 5.5 6.88 (about 0.11 per run), Haiku 5.5 0.70 (about 0.012 per run). The pilot (40 runs, USD 2.25) and a discarded first pilot (USD 1.59, every CLI call blocked on approval because the session hook names the CLI by its absolute path) are not in these numbers.
- Catch rate over planted runs: T1 44/48 (0.92), T2 42/48 (0.88). Size: 60 ADRs 43/48, 240 ADRs 43/48. Position: early 43/48, late 43/48. Wording: same 48/48, paraphrased 38/48. Arm: prompted 48/48, unprompted 38/48. Model: Haiku 48/48, Sonnet 38/48.
- False alarms: 0 in 120 runs (no supersede and no listed number outside the plant, controls included). Read before the first write: 120/120.
- Every miss (10) is Sonnet, unprompted, paraphrased: T1 at 60 and at 240 caught 1 of 3 each, T2 at 60 and at 240 caught 0 of 3 each. With the paraphrase and the request for `CONFLICTS.txt`, Sonnet listed the plant every time; Haiku caught every planted run in both arms.
- Decision rule, pooled: unprompted catch rate 0.79 at 240 ADRs, prompted 1.00 at 240, unprompted 0.79 at 60, so the rule says build O2 (unprompted at 240 is below the prompted arm). Per model: Sonnet 0.58 / 1.00 / 0.58, build O2; Haiku 1.00 / 1.00 / 1.00, do not.
- Reading: in this range the catch rate does not fall with size (the same 0.79 at 60 and at 240); the loss is a paraphrased conflict that the unprompted session never looks for. That gap is what candidates printed at `write new adr` (O2) target, since they need no request from the user.
- Upper bound: the isolated runner read before every first write (120/120), against about 16 of 140 field sessions (issue 0149's Outcome), so field catch rates are likely lower.
- Other observations: 13 runs (Sonnet 11, Haiku 2) left the new ADR's `description` as the template placeholder, so `check` was red; those are scored incomplete and do not change the catch. 9 runs edited a CLI-owned field by hand.
- Limits: one seed, one plant per type and wording, three runs per cell. The reworded T1 paraphrase says "router" where the task says "routing", a shared stem no filler record uses (a reviewer nit kept so the runs stay comparable). Models write scratch bodies to fixed paths such as `/tmp/body.md`, so runs must not go in parallel.
