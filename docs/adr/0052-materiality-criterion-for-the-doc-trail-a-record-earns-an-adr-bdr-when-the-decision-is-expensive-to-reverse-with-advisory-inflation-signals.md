---
type: ADR
title: "Materiality criterion for the doc trail: a record earns an ADR/BDR when the decision is expensive to reverse, with advisory inflation signals"
description: the doc trail is gated by materiality, not by layer — a decision earns its own ADR/BDR only when it is expensive to reverse; strict refuses a missing MATERIAL record, not a record per behavioral diff; scorecard emits advisory inflation signals and check a ~300-word decision-prose budget
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:34:31Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0052. Materiality criterion for the doc trail: a record earns an ADR/BDR when the decision is expensive to reverse, with advisory inflation signals

## Context

Refusal trigger 7 keys on the *kind* of change (structural → ADR, behavioral → BDR), never on the *materiality* of the decision. A rule with a hard stop and no instrument resolves toward over-production: the safest way to be compliant is to write the record. On one strict-mode consumer that produced 142 ADRs in three months, 139 with a same-numbered BDR — pairing by rule, not by need. The skill gives the agent nothing to say "no" with.

## Decision

We will gate the doc trail on **materiality, not layer**. A decision earns its own ADR when it is expensive to reverse — it changes a stated invariant or constitution article, a public contract or schema, a dependency direction or module boundary, a pinned dependency, or it reverses a prior ADR. A BDR is earned when a new observable contract is introduced or an existing one changes for consumers. A decision cheap to reverse lives in a `## Decision` section of its issue. The test: "would a future reader pay for rediscovering this?"

`strict` becomes "the right record for the materiality": a *material* decision shipped without its record is a blocked task; a non-material decision recorded only in its issue is compliant in every mode. Falling back to `lite` is not the fix — that drops the whole trail; this is a filter.

The instrument-first half is advisory and never gates: `scorecard` emits inflation signals (ADR count and recent count, ADR:BDR pairing ratio, supersession count, proposed-stale count) and `check` a `SIZE` decision-prose budget (~300 words across Context/Decision/Consequences).

## Consequences

**Easier / gained:**
- The agent has a defensible "no", and drift toward one-record-per-change is observable before it reaches the corpus.

**Harder / accepted trade-offs:**
- Materiality is still a judgement with no oracle; the signals make the cost of getting it wrong visible, they do not decide it.

**Follow-ups:**
- Revisit whether any inflation signal earns promotion from advisory to a gate once its false-positive rate is known.

## Verification

**Implementation impact:** `living-docs-core/src/commands/scorecard/inflation.rs`, `living-docs-core/src/check/size.rs`, `skills/living-docs/rules/adr-conventions.md`, `skills/living-docs/rules/bdr-conventions.md`, `skills/living-docs/rules/enforcement-modes.md`.

**Verification criteria:**
- `scorecard` prints the inflation line and `check` the decision-prose `SIZE` advisory, both without changing the exit code; `check docs` on this repo stays green.
- Trigger 7 and the two convention files state the materiality criterion with a worked example that stays in an issue and one that earns an ADR.
