---
type: ADR
title: blocked_by fails check only on a dangling issue number, a self-reference or a cycle; other items are advice
description: "Supersede ADR 0092 after a 28-bundle QA pass: keep the certain dependency defects as violations, report non-numeric items as advisories, accept them at write time and split scalar comma lists"
owner: Evaldo Klock
status: Superseded
supersedes: 0092
superseded_by: 0088
timestamp: 2026-11-15T22:15:41Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0088](0088-every-blocked-by-finding-is-advice-and-none-fails-check.md). Run `living-docs read records` for what is in force.

# 0086. blocked_by fails check only on a dangling issue number, a self-reference or a cycle; other items are advice

## Context

[ADR 0092](/adr/0092-blocked-by-names-local-issue-numbers-and-check-fails-on-a-dangling-dependency-a-self-reference-or-a-cycle.md) made every `blocked_by` item a local issue number: `write set` refused anything else, and `check` failed on an item that is not a number, names no issue, names the issue itself, or closes a cycle. It rested on a probe of two bundles (this one and relent) in which every item was a bare issue number. [Issue 0125](/issues/0125-implement-adr-0082-blocked-by-validation-in-check-write-time-refusal-and-renumber-rewrite.md) implemented it.

A QA pass on 2026-11-15 ran the new `check` against all 28 living-docs bundles on the owner's machine and compared it with the released 0.28.0 binary. 24 bundles gained nothing; four gained violations from the new pass: agent-memory 143, jira-cli 6, active-collab-cli 1, herdr-workspaces 1. In agent-memory the items were `ADR NNNN` (68), `#N` tracker numbers (36), `BRD N` (20), numbers naming no issue (16) and `Slice N` (2): authors use `blocked_by` to wait on an ADR being accepted or on a tracker item, not only on local issues. The jira-cli and active-collab-cli hits were valid numbers written as a scalar comma list (`blocked_by: 0045, 0047`), which the pass read as one item — a defect, not a design question. Under ADR 0092 every commit touching one of those issues fails the gate, which the owner judged blocking.

## Decision

We will fail `check` only on dependencies that are certainly wrong, and report every other item as advice.

- **Violations** (unchanged from ADR 0092): a numeric item that names no issue of the bundle, an issue that lists its own number, and a cycle of issues, naming every member.
- **Advisory**: an item that is not a decimal number (`ADR 0077`, `#21`, free text). It is reported per issue as `blocked_by` advice, never fails `check`, and is left out of the dependency graph.
- **Write time**: `write set <issue> blocked_by` accepts any item again; numeric items are still written so they read back as numbers.
- **Parsing**: a scalar comma list (`blocked_by: 0045, 0047`) is split into items like a flow list.
- **Renumber**: unchanged — only numeric items equal to the old number are rewritten.
- Waves derived from dependencies use numeric items only; a non-numeric item never blocks a wave.

Rejected alternatives:
- **Keep ADR 0092 strict.** It keeps one referent, but blocks four real bundles until their authors migrate items that express a real wait (an ADR to be accepted, a tracker item) to another key with no home for it.
- **Everything advisory for one release, then strict.** It buys time but ends in the same block; the evidence says the non-numeric uses are intentional, not legacy.
- **Revert to free text with no check.** It drops the cycle and dangling-number protection that has no false positives, and the renumber rewrite that needs a numeric referent.
- **Resolve `ADR NNNN` items as typed references.** It would model one of several observed forms (tracker numbers and free text would still need a rule) and widen the key's contract; an issue already declares the ADR it implements in its body ([ADR 0079](/adr/0079-stale-proposed-counts-only-an-issue-that-declares-it-implements-the-adr.md) keeps that the single link).

## Consequences

**Easier / gained:**
- No bundle on the machine gains a blocking violation from free-form items; the scalar comma defect is gone.
- The certain defects (dangling numbers, self-references, cycles) still fail the gate.

**Harder / accepted trade-offs:**
- `blocked_by` again has two kinds of item, and only numeric ones take part in validation, waves and renumbering.
- Non-numeric items produce advisories that some bundles will carry for a long time.

**Follow-ups:**
- An issue to implement the advisory, the scalar split and the restored write-time acceptance, proved by rerunning the 28-bundle comparison.

## Verification

**Implementation impact:** `living-docs-core/src/check/dependencies.rs`, `living-docs-core/src/commands/set/issue_keys.rs`, `skills/living-docs/rules/issue-workflow.md`, their tests.

**Verification criteria:**
- `blocked_by: [ADR 0077, "#21"]` produces two advisories and `check` exits 0; `blocked_by: 0045, 0047` resolves both numbers.
- A dangling number, a self-reference and a cycle still fail `check`.
- `write set issue/NNNN blocked_by "ADR 0077, 12"` succeeds.
- Rerunning `check` over the 28 bundles adds no violation relative to 0.28.0 except numeric dangling items, self-references and cycles.
