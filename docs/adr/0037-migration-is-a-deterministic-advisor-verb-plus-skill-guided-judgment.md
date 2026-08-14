---
type: ADR
title: Migration is a deterministic advisor verb plus skill-guided judgment
description: living-docs migrate scans a bundle (or its absence) and prints an ordered adaptation plan split into RUN steps (mechanical CLI commands) and AUTHOR steps (judgment the authoring model owns); the tool never edits records itself, and rules/migration.md governs the judgment half.
status: Proposed
timestamp: 2026-08-14T08:12:23Z
---

# 0037. Migration is a deterministic advisor verb plus skill-guided judgment

## Context

ADR 0035 (EARS requirement IDs + traceability) and ADR 0036 (CLI-owned architecture
views) changed what a current bundle looks like, so two adaptation scenarios now recur:
a bundle authored under an older Living Docs organization (single `architecture.md`,
kind-less views, ID-less PRD requirements, hand-maintained table indexes), and a project
entirely outside Living Docs that wants to adopt it. Detection of both is mechanical — a
machine checks shapes better than prose — but most of the repair is not: splitting an
architecture file into views, assigning a view's kind, rewriting requirements in EARS
form, and back-filling decision records are judgment the determinism boundary (ADR 0001)
forbids the tool to perform.

Epistemic type: **Judgment** — where to draw the tool/model line is a trade-off with no
experiment available; no number is attached. External critic: **pending** (real
migrations qualify).

## Decision

We will add `living-docs migrate`: a read-only advisor that scans the bundle (or detects
its absence) and prints one ordered adaptation plan with two step shapes — `RUN` steps
naming the exact mechanical CLI commands (`index`, `fmt`, `hooks install`, `check`) and
`AUTHOR` steps naming the judgment work with the record, the gap, and the governing ADR.
On a missing bundle it prints the `ADOPT` bootstrap sequence instead. The verb never
writes or edits anything; the judgment half is governed by the skill topic
`rules/migration.md`, which instructs the authoring model through the `AUTHOR` steps
(brownfield rule preserved: confirm each back-filled decision with the user, never infer).

## Consequences

**Easier / gained:**
- Legacy-bundle drift and adoption both become one command that yields a complete, ordered plan an agent can execute directly.
- Detection lives beside the invariants it mirrors (`check`'s modules are reused, e.g. requirement-ID scanning), so plan and gate cannot disagree.
- Read-only means `migrate` is safe to run anywhere, any time — it is a diagnosis, not a mutation.

**Harder / accepted trade-offs (the declared sacrifice):**
- The tool does not apply even the safe mechanical steps itself. Steelman of the rejected
  alternative — `migrate --apply` running `index`/`fmt` in one shot; rejected for this
  slice because the executing agent already runs printed commands verbatim, and an
  applying verb needs its own transactional/rollback story before it earns existence.
  *(Amended by [ADR 0040](/adr/0040-migrate-apply-is-a-cli-front-transaction-over-the-mechanical-subset.md):
  the transactional story now exists — `--apply` in the CLI front; the core advisor
  stays read-only.)*
- A skill-only instruction page without a verb was rejected on the standing rule: a
  constraint without an instrument is a vibe — shape detection is mechanical and belongs
  in the binary.
- Plan text is a contract surface: agents will parse `RUN`/`AUTHOR`/`ADOPT` prefixes, so
  wording changes are breaking changes and the shapes are pinned by tests.

**Follow-ups:**
- `rules/migration.md` (skill topic) + SKILL.md router bullet.
- A future `--apply` for the mechanical subset may supersede the first sacrifice.

## Verification

**Implementation impact:** `living-docs-core/src/commands/migrate.rs` (new),
`cli/src/{args,main}.rs` + `cli/src/commands/migrate.rs`,
`skills/living-docs/rules/migration.md`.

**Verification criteria:**
- A bundle with a root `architecture.md`, a kind-less view, a non-Draft ID-less PRD, and
  a table-format ADR index yields exactly one `AUTHOR`/`RUN` step per finding, in plan
  order; a current bundle prints "nothing to adapt"; a missing bundle prints the `ADOPT`
  sequence.
- Fitness function: `commands::migrate` unit tests pin the three scenarios and the
  step-prefix contract.

# References

[1] [ADR 0035](/adr/0035-requirement-ids-are-prd-scoped-ears-statements-and-check-traces-bdr-coverage.md)
[2] [ADR 0036](/adr/0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md)
