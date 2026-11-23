---
type: ADR
title: The record lifecycle is one transition table that set status and supersede consult before any write
description: Set status and supersede consult one lifecycle table and refuse a re-supersede, a cycle, a retired successor and a status change on a superseded record.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-23T15:41:19Z
---

# 0104. The record lifecycle is one transition table that set status and supersede consult before any write

## Context

A record's lifecycle is implicit today: `write set status` accepts any value in the type's vocabulary and reserves `Superseded` for `write supersede` ([ADR 0029](0029-the-status-vocabulary-is-per-doc-type-sourced-from-one-doctypespec-field-not-one-global-list-validated-against-every-template-s-own-dialect.md)), and `write supersede` accepts any pair of records. Issue 0154 found three supersede transitions that exit 0 and leave the log wrong (binary v0.31.0): a re-supersede that rewrites an existing successor link and leaves `check` red; a supersede that closes a cycle, so no record of the chain is in force while `check` stays green; and a supersede by a retired record, which drops the decision from the in-force view. A design review on 2026-11-23 rejected a stateful component to hold the lifecycle; git stays the log and the CLI stays stateless ([ADR 0001](0001-living-docs-cli.md), [ADR 0059](0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md)).

## Decision

We will hold the record lifecycle in one explicit transition table in the core, consulted by `write set status` and `write supersede` before any file is written. An illegal transition exits 2, names the rule it breaks and the records involved, and changes no file.

- `write set status <record> <value>`: legal when the value is in the type's vocabulary and the record is not `Superseded`. Every move inside the vocabulary stays legal, reopening included (`Deprecated → Accepted` for an ADR, `closed → open` for an issue). `Superseded` is never set this way, as today. A `Superseded` record is refused, since a new status would leave its `superseded_by` link describing a replacement that no longer holds.
- `write supersede <old> <new>`: legal when `old` has no successor yet, `new` is not retired (not `Superseded` and not a terminal status of its type), and following `superseded_by` from `new` never reaches `old`. A re-supersede, a cycle and a retired successor are each refused with their own message. A `Proposed` successor is legal: the core counts it in force.
- `write set status Deprecated` on a record that superseded another stays legal (owner, 2026-11-23): retiring a decision with no replacement is an ordinary move, so a chain may end in a retired record. The chain rule binds `write supersede` only.
- `check` adds one advisory ([ADR 0095](0095-a-new-check-rule-starts-as-an-advisory-and-becomes-a-violation-only-by-a-later-adr-that-counts-it-over-real-bundles.md)), `supersede-cycle`, for a cycle of `superseded_by` links a hand edit can still produce. A chain ending in a retired record is not reported.

The table is a contract: each refusal is part of its verb's exit-code behavior, and loosening or tightening a row needs a later ADR.

## Consequences

- Calls that exit 0 today start failing with exit 2: a re-supersede, a supersede that closes a cycle, a supersede by a retired record, and `write set status` on a `Superseded` record. A script that relied on any of them breaks loudly instead of writing a wrong log.
- Undoing a supersede is not a status change. It stays a hand edit of both records, which `check` then judges.
- The exhaustive write-sequence test of issue 0149, widened in issue 0154, is the fitness function: every accepted step leaves each successor link unchanged and the chain acyclic, and every refused step changes no file.
- The CLI still never decides which decision is right. It refuses only transitions that would make the log contradict itself.
