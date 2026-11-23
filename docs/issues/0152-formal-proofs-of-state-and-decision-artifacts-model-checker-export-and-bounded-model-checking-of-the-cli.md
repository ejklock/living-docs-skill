---
type: Issue
title: Formal proofs of state and decision artifacts, model-checker export, and bounded model checking of the CLI
description: check proves state views and decision tables, a state view exports to a model-checker spec for the project's CI, and the CLI's own pure functions get bounded model checking.
status: open
timestamp: 2026-11-23T05:14:53Z
---

## 0152. Formal proofs of state and decision artifacts, model-checker export, and bounded model checking of the CLI

Follows the domain records of [ADR 0099](../adr/0099-domain-records-are-a-registry-type-in-docs-context-nested-in-folders-by-bounded-context.md) and the generated views of [ADR 0101](../adr/0101-generated-view-regions-carry-a-hash-of-their-inputs-stamped-by-manage-view-splice-and-checked-for-staleness.md): the engineering artifacts a bundle keeps (state diagrams, decision tables) are small formal models, and the CLI itself has pure functions whose safety rests on universal properties.

### Scope

- Proofs over a bundle's artifacts inside `check`, as advisories: a state view's reachability, dead states and dangling transitions; a decision table's completeness and consistency.
- A deterministic export of a state view to a model-checker specification, which the project's own CI runs; the binary never runs the checker.
- Bounded model checking of the CLI's own pure functions (path validation, glob pruning, inputs hash, slug, marker parser), in a manually dispatched CI job, with nothing added to the binary.

Kept: the determinism boundary (no model inside the binary, no external tool run by `check`), and every existing rule.

### Decision

- Formal verification (owner, 2026-11-23): all three arms: proofs of state and decision-table artifacts in `check`; export of state views to a model-checker specification for the project's CI; bounded model checking of the CLI's own pure functions. Not taken: any one arm alone (the session recommended the `check` proofs first, the export only on demand).

### Acceptance

- To be set by the ADRs this issue produces, after a design review.

### Plan

- S1: design review of the three arms, then one ADR per arm | files: docs/adr | after: none
