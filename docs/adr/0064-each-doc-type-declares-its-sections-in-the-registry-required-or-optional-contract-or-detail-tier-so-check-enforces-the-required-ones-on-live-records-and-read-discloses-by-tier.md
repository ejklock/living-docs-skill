---
type: ADR
title: "Each doc type declares its sections in the registry: required or optional, contract or detail tier, so check enforces the required ones on live records and read discloses by tier"
description: A per-type section schema in the DocTypeSpec registry marks each body section required or optional and contract or detail; check fails a live record missing a required section, and read gains a --contract level between the one-line index and --full.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-05T20:53:01Z
---

# 0064. Each doc type declares its sections in the registry: required or optional, contract or detail tier, so check enforces the required ones on live records and read discloses by tier

## Context

The registry ([ADR 0027](/adr/0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md)) owns every rule keyed by doc type, but it says nothing about a record's body sections. Two gaps follow.

First, no section is required. `check` reports an unfilled `{{SLOT}}` and accepts a deleted one, so an issue with no `### Acceptance` passes the gate. The template hints mark some sections optional ("remove the section when the issue decides nothing", "for a large task"); nothing makes the others mandatory.

Second, `read` discloses in two steps only: one line per record, or `--full` with the whole body. An agent orienting on an issue needs what the change is and how it is known done; it does not need the delivery plan or the outcome until it executes or audits the work. A full body mixes both. The owner weighed a separate `plan` record type to split them; the underlying need is disclosure by level, not a second record. The deprecated ADR 0050 already named progressive tiers; the shipped `read` kept only two of them.

In this repository 47 of 51 issues carry `Scope` and `Acceptance`; the four that do not are closed history.

## Decision

We will add a section schema to each `DocTypeSpec`: an ordered list of named body sections, each marked **required** or **optional** and placed in the **contract** or the **detail** tier.

- **`check`** fails a live record (status neither terminal nor retired) whose body lacks a required section. A section matches by heading text, case-insensitive, at any heading level below the title. Terminal and retired records are history and are never checked, so a brownfield bundle does not go red on its closed records.
- **`read --contract`** prints, per record, the preamble under the title and every section in the contract tier, and omits detail-tier sections. A heading the schema does not name is shown as contract: the tool never hides what it does not know. `--contract` and `--full` are mutually exclusive; the one-line index stays the default; `--full` is unchanged.
- **The issue row** is the first populated schema: `Scope` and `Acceptance` required and contract; `Decision` optional and contract; `Plan` and `Outcome` optional and detail. Every other type starts with an empty schema, which requires nothing and discloses everything as contract, until its own issue fills it.

Rejected alternatives:

- **A `plan` record type.** It separates the plan by file, at the cost of a second record per change, a link the gate must keep in step, and a record that is stale by construction once the work merges. It also leaves the missing-section gap open.
- **Keep the status quo.** Required sections stay unenforced, and the agent keeps reading the plan with the contract.
- **Tier markers inside the template or the body** (an HTML comment per heading). An author can delete or edit the marker, so the tier would drift per record; the registry is the single home for a per-type rule (ADR 0027).
- **Change `--full` to mean contract only.** It breaks the existing contract of `read --full` for every caller; an additive `--contract` level does not.

## Consequences

**Easier / gained:**
- The gate catches a live issue without acceptance criteria.
- An agent can read the in-force contract of every record without the plans, and open the file for the detail it needs.
- Adding a section rule to another type is a registry row edit, not new code.

**Harder / accepted trade-offs:**
- Heading text becomes part of the contract: renaming `### Acceptance` to `### Done when` on a live issue fails `check`.
- A bundle in another repository with live issues that lack `Scope` or `Acceptance` goes red after upgrading; `check --changed-files` ([ADR 0062](/adr/0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md)) limits that to the records a commit touches.
- `read` gains a third level and the flag surface grows by one.

**Follow-ups:**
- One issue per remaining doc type (ADR, PRD, research, constitution) to populate its section schema.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs` (schema field and the issue row), a new `check` pass beside `check/placeholder.rs`, `living-docs-core/src/commands/effective.rs` and its render module (contract level), `cli/src/commands/read.rs` (the flag), the issue template hints, and the `issue-workflow` and `doc-trail` guide topics.

**Verification criteria:**
- `living-docs new issue` output with every slot filled passes `check`; the same record with `### Acceptance` removed fails it while `status: open`, and passes once `status: closed`.
- `read --contract` over an issue prints `Scope` and `Acceptance` and omits `Plan` and `Outcome`; `read --full` output is byte-identical to its output before this change.
- `living-docs check` over this repository's `docs/` stays green.
- Fitness function: a core test asserts that every section a registry row marks required also appears as a heading in that type's template, so `new` never scaffolds a record that fails its own schema.
