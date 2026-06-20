---
type: BDR
title: <Short behavior title>
description: <One sentence — the observable behavior this specifies.>
status: Draft               # Draft | Accepted | Implemented | Superseded
superseded_by:              # NNNN, set when a later BDR replaces this one
tags: []
timestamp: <ISO 8601 datetime>
---

# NNNN. <Short behavior title>

<!-- Status lives in frontmatter (`status`), not a body line. -->

## Context

<What observable behavior is being specified? What user need or system requirement
demands it? Link the PRD or ADR that spawned this BDR, and any issue that tracks the
work, bundle-relative: [PRD](/prd/NNNN-<slug>.md), [ADR](/adr/NNNN-<slug>.md).>

## Behavior

```mermaid
flowchart TD
    A[Trigger / input] --> B[Step]
    B --> C{Decision}
    C -->|yes| D[Outcome A]
    C -->|no| E[Outcome B]
```

<Replace the diagram above with a flowchart or sequence diagram that shows the full
observable flow. Use Mermaid only — no images, no ASCII art.>

## Textual Description

<Prose form of the behavior. Describe what the system does from the outside: inputs,
outputs, side effects, error paths. Write as if the code does not exist yet — what an
observer would verify by watching the running system.>

## Scenarios

Each scenario is written to convert verbatim into the project's behavioral regression
suite. Number from 1; do not skip numbers.

**Scenario 1: <happy-path name>**

- Given <initial state or precondition>
- When <the trigger or action>
- Then <the observable outcome>

**Scenario 2: <edge or error case>**

- Given <initial state or precondition>
- When <the trigger or action>
- Then <the observable outcome>

## Test Design

How this behavior is tested — the single home for the **how** (an execution issue links here, never copies). Each Given/When/Then above is *one example, not the spec*: expand each into the matrix below and name what every row PROVES. A row with no "proves" is a smell (likely vacuous).

| Case | Level | Input / scenario | Asserts (observable) | Proves |
|---|---|---|---|---|
| Happy path | <unit/integration/e2e> | <the scenario above> | <output/state a caller sees> | success contract on a valid input |
| Boundary | <…> | <edge: 0 / max / empty / null> | <…> | off-by-one / limit handled |
| Equivalence | <…> | <one per valid+invalid class> | <…> | each input class handled |
| Error path | <…> | <failure mode> | <observable error + no side effect> | failure is a contract |
| Property | <…> | <invariant, e.g. `decode(encode(x))==x`> | <holds over generated inputs> | uncheatable by hardcoding |

Rules: behavior-spec-first (write the test, watch it fail, then implement); assert observable behavior, not internal calls/private fields; mock only out-of-process deps, never the DB. A per-behavior *strategy decision* with a rejected alternative (why this level/technique, or a deviation from the standing bar) is a **test-strategy ADR** linked under Related — not duplicated here.

## Related

- PRD: [/prd/NNNN-<slug>.md](/prd/NNNN-<slug>.md)
- ADR: [/adr/NNNN-<slug>.md](/adr/NNNN-<slug>.md)
- Test-strategy ADR (if any): [/adr/NNNN-<slug>.md](/adr/NNNN-<slug>.md) — the *why* behind a non-default level/technique or a bar deviation.
- Issues: [/issues/NNNN-<slug>.md](/issues/NNNN-<slug>.md)
