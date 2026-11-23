---
type: ADR
title: manage view exports a flat state view as a TLA+ module for the project's model checker
description: manage view <view> --export tla prints a byte-stable TLA+ module of a flat state view; the author's temporal properties extend it and the project's CI runs TLC.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-23T08:14:39Z
---

# 0103. manage view exports a flat state view as a TLA+ module for the project's model checker

## Context

The `state-machine` advisory of [ADR 0102](0102-check-proves-state-views-and-marked-decision-tables-as-advisories.md) proves structural properties of a lifecycle. It cannot prove behavior over time, such as "a paid order never returns to pending", which a model checker answers from a temporal property the author writes. The owner chose on 2026-11-23 (issue 0152) to export a state view to a model-checker specification for the project's own CI, in TLA+ only; the binary never runs a checker ([ADR 0001](0001-living-docs-cli.md)), as it never runs a view's extractor ([ADR 0101](0101-generated-view-regions-carry-a-hash-of-their-inputs-stamped-by-manage-view-splice-and-checked-for-staleness.md)).

## Decision

We will add `living-docs manage view <view> --export tla`, which prints to standard output a TLA+ module for a state view whose diagram is flat, and refuses a diagram with composite states or concurrent regions, naming them.

- The module is named after the view's slug with hyphens turned into underscores, as TLA+ requires the module name to match the file name. It declares `VARIABLE state`, the set `States`, the set `Terminal`, `Init`, one action per transition in the diagram's source order, `Next` as their disjunction plus a stuttering step on each terminal state, `TypeOK`, and `Spec == Init /\ [][Next]_state /\ WF_state(Next)`; a transition's label is kept as a comment, since guards are free text.
- The output is byte-identical for the same view and CLI version: no timestamp, no version line, transitions in source order.
- The author writes the temporal properties in a separate module that `EXTENDS` the generated one, with its own `.cfg`; the skill ships a CI recipe that generates the module, never commits it, and runs TLC from a hash-pinned `tla2tools.jar`.

These operator names are a contract the author's property modules depend on.

## Consequences

- A project checks invariants and liveness of its documented lifecycle in its own CI, against the diagram the bundle keeps.
- The model is an over-approximation: guards and data are not modelled, so a property that depends on them is the author's to encode.
- Renaming a generated operator breaks every property module that names it; it needs a later ADR.
- A JVM enters the consumer's CI, not the binary.
- Alloy is not exported; it waits for a consumer that asks.
