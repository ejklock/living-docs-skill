---
type: Issue
title: Make the record lifecycle an explicit transition table and refuse illegal supersede transitions
description: Three illegal supersede transitions pass today (re-supersede, cycle, retired successor); one transition table in the core, consulted by set and supersede, plus chain checks, closes them without any stateful component.
status: open
timestamp: 2026-11-23T12:37:08Z
---

## 0154. Make the record lifecycle an explicit transition table and refuse illegal supersede transitions

A design review on 2026-11-23 asked whether living docs need a stateful component, such as a durable single-writer object, to track code. The verdict was no: git already is the ordered, durable log (commits as events, the tree as state), and a stateful actor closes none of the stale-decision gaps of issue 0153 while breaking the constitution's "no host tools, no daemon, no database" and the determinism of ADR 0001. It found instead that the record lifecycle is implicit across verbs, with no transition relation, and that three illegal supersede transitions pass today (binary v0.31.0, scratch bundles):

- D1, re-supersede: `write supersede 0002 0003` then `write supersede 0002 0004` exits 0 and leaves `check` red with an unmirrored link on 0003.
- D2, cycle: `write supersede 0001 0002` then `write supersede 0002 0001` exits 0, `check` is green, and neither record is in force.
- D3, retired successor: superseding by a `Deprecated` record exits 0, `check` is green, and the decision leaves the in-force view.

### Scope

- One explicit transition table in the core that `write set status` and `write supersede` consult, refusing an illegal transition before any write.
- `check` rules for the states a hand edit can still produce: a supersede chain is acyclic and ends in a record in force (born advisories, ADR 0095).
- The exhaustive write-sequence test of issue 0149 widened so D1 to D3 are red before the fix.

Kept: the stateless CLI over the `.md` tree; every status vocabulary; `Deprecated → Accepted` and `closed → open` stay legal.

### Decision

- No stateful component (owner, 2026-11-23): the lifecycle becomes an explicit transition table in the core, with no daemon, actor or database. Not taken: keeping the implicit lifecycle; a persistent single-writer actor, which would supersede ADR 0059 and amend the constitution.
- Reopening stays legal (owner, 2026-11-23): `Deprecated → Accepted` for an ADR and `closed → open` for an issue are legal transitions. Not taken: only reopening an issue; terminal statuses as absorbing states.
- The refusal changes a verb's exit-code contract (calls accepted today start failing), so the table and its refusals are decided by an ADR before code.

### Acceptance

- The write-sequence test, with a third ADR as a supersede target in its alphabet and a contract part requiring that a supersede leaves its successor in force, is red on D1, D2 and D3 before the fix and green after.
- `write supersede` refuses D1, D2 and D3 with exit 2 and changes no file; `write set status` consults the same table.
- `check` reports a supersede cycle and a chain ending in a retired record.

### Plan

- S1: widen the write-sequence test so D1 to D3 are red | files: fs-store/tests/write_sequences | after: none
- S2: the ADR for the transition table and its refusals | files: docs/adr | after: S1
- S3: the transition table in the core, consulted by set and supersede, and the two check rules | files: living-docs-core/src/doc_type, commands/set, commands/supersede, check | after: S2
