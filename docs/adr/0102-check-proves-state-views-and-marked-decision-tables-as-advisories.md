---
type: ADR
title: check proves state views and marked decision tables as advisories
description: check reports unreachable states, dead ends and multiple initial transitions in state views, and missing, contradictory or shadowed cases in marked decision tables of domain records.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-23T08:14:35Z
---

# 0102. check proves state views and marked decision tables as advisories

## Context

A bundle keeps two kinds of engineering artifact that are small formal models: a state view ([ADR 0036](0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md), `kind: state`, a Mermaid `stateDiagram`) and, since [ADR 0099](0099-domain-records-are-a-registry-type-in-docs-context-nested-in-folders-by-bounded-context.md), the business rules of a domain record. Today `check` only parses the Mermaid ([ADR 0013](0013-mermaid-validation-runs-in-process-via-merman-core-not-a-docker-mermaid-cli-shell-out.md)) and reads nothing of a rule. A lifecycle with an unreachable state or a dead end, or a decision table with a missing or contradictory case, passes in silence, although over a finite model both questions are decidable. The owner chose on 2026-11-23 (issue 0152) to prove them inside `check`.

## Decision

We will add two advisories to `check`, each born an advisory under [ADR 0095](0095-a-new-check-rule-starts-as-an-advisory-and-becomes-a-violation-only-by-a-later-adr-that-counts-it-over-real-bundles.md).

- `state-machine`: for every `stateDiagram` fence in a view with `kind: state`, built from the model merman-core already parses, it reports a state not reachable from the initial state, a dead end (a state that is not final, has no outgoing transition, and has no enclosing composite state with one), and a scope with more than one initial transition. A diagram with concurrent regions (`--`) or with no root initial state gets one advisory saying what was not proved; notes are ignored; choice, fork and join are plain states.
- `decision-table`: a table is a decision table only when the line before it is `<!-- decision-table -->` or `<!-- decision-table: first -->`, under a `###` rule heading of a domain record. Every column but the last is a condition whose header declares its values, `name (v1, v2, ...)`; a cell holds one value, a comma-separated set, or `-` for any; the last column is the outcome, compared as trimmed text. With the default policy every combination of condition values must match a row, and no combination may match two rows with different outcomes; with `first`, every combination must match a row and no row may be fully shadowed by earlier rows. A cell value outside its declared domain is reported. A table over 4096 combinations is reported as too large to prove.

The grammar of the marker and of the header is a contract kept in consumer files. Nothing here runs an external tool or reads code.

## Consequences

- A lifecycle drawn in a state view and a decision rule written as a marked table are proved complete and consistent at commit, with no tool installed.
- A terminal state written without `--> [*]` reads as a dead end; the advisory says to add `X --> [*]`. This is the commonest way to write a lifecycle, so the first counts over real bundles will be high.
- The state check depends on merman-core's model, pinned at one version; a fixture test guards the shapes and identifiers it reads.
- No existing table is a decision table until its author adds the marker, so promotion counts start at zero.
- The CLI never decides what a rule means, which outcome is right, or whether a state should exist: it reports what the declared model makes unreachable, stuck, missing or contradictory.
- Fitness functions: a fixture per finding raises it; a complete, consistent table and a well-formed state diagram raise nothing; an unmarked table is never read.
