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

### Acceptance

- Ten scenarios (T1, T2 and a control with no plant; 60 and 240 ADRs; the conflicting record worded with the task's words and paraphrased), two arms (unprompted task; task that asks for conflicting record numbers in `CONFLICTS.txt`), two models, three runs each, scored by `expect.sh`: caught means the planted record is superseded by or linked from the new record (unprompted) or listed (prompted); the control fails on any supersede or listed number.
- An aggregate over the runs' `score.json`: catch and false-alarm rate per type, size, wording, arm and model, plus `read_before_first_write`, recorded in this issue's Outcome with the cost of the runs.
- The scenario generator is seeded and deterministic; the binary is not changed.

### Plan

- S1: the scale eval — seeded generator, T1/T2/control scenarios, prompted and unprompted arms, scorer, aggregator, and one recorded run | files: evals/ | after: none
- S2: decide O2 from S1's numbers by the fixed rule; if built, rerun S1 against it | files: to be planned | after: S1
- S3: decide O1 (amend relation) from the T2 numbers, by ADR | files: docs/adr | after: S1
