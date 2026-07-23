---
type: ADR
title: A living-docs CLI that owns the deterministic layer of doc authoring
description: Introduce a `living-docs` CLI that mechanizes the non-judgment parts of authoring a doc — scaffolding, number allocation, index regeneration, supersede wiring, and conformance checking — leaving rationale prose to the authoring model. The `check` subcommand doubles as the doc-gate that makes a future cheap-model rendering split safe.
status: Accepted
tags: [authoring-cost, cli, determinism, doc-gate, tooling]
timestamp: 2026-07-14T00:00:00Z
---

# 0001. A living-docs CLI that owns the deterministic layer of doc authoring

## Context

Authoring a Living Docs artifact (ADR, PRD, BDR, issue, constitution) bundles three
separable activities into one act by a single agent:

1. **Decide** — the epistemic type, the forces at play, the chosen option, the rejected
   alternatives and *why*. This is irreducible judgment.
2. **Render** — expand a settled decision into the OKF/MADR house style: prose for
   Context / Decision / Consequences.
3. **Mechanics** — allocate the next `NNNN`, fill frontmatter (`type`, `status`,
   `timestamp`), write the file at the right path, add the index row (Active/Superseded
   split), wire `supersedes` / `superseded_by` bidirectionally, and validate that the
   result conforms (OKF frontmatter, one-home/indexed invariants, supersede-chain
   integrity, Mermaid validity, no broken links).

Today an agent does all three by hand, every time. Two problems follow. First, the
mechanical layer (3) is deterministic yet is paid in model tokens and is error-prone by
hand — a mis-numbered ADR, a stale index, a one-directional supersede link, a frontmatter
key drift. Second — the motivating force — there is interest in moving the **render**
layer (2) to a cheaper model to cut authoring cost (the cost lives in the prose output
tokens). That split is only safe if a **deterministic verifier** can catch structural
drift a cheaper renderer might introduce; without one, a cheap renderer has no guardrail
and the boundary between "decided" and "invented" blurs.

The repo already ships the validating half of layer 3 as scripts —
`skills/living-docs/scripts/lint-docs.sh` and `lint-mermaid.sh` — but there is no
first-class, generative front door (`new`, `index`, `supersede`, `next`), and the
existing checks are not framed as a gate a review step or a cheaper authoring model can
consume.

## Decision

We will add a **`living-docs` CLI** that owns the **deterministic layer of doc authoring
only** — never the rationale. It is the determinism-ratchet applied to docs: push every
mechanical, template-fillable step out of the model and into a script, and expose the
conformance check as a gate.

**Subcommands.**

Generative (zero model tokens):
- `living-docs new <type> "<title>"` — scaffold from `templates/<type>.md`, allocate the
  next sequential `NNNN`, fill `type` / `status: Proposed` / `timestamp`, slugify the
  title, and write to the type's canonical path. Body sections are left as template
  placeholders for the authoring model to fill.
- `living-docs index [<type>]` — regenerate `docs/<type>/index.md` from each file's
  frontmatter, including the ADR `## Active` / `## Superseded` split by `status`.
  Idempotent: regenerating twice yields no diff.
- `living-docs supersede <old-NNNN> <new-NNNN>` — bidirectional wiring: set the old
  record's `status: Superseded` + `superseded_by`, set the new record's `supersedes`.
  Does not touch the old record's Context/Decision/Consequences (that is history).
- `living-docs next <type>` — print the next available number (for scripting).

Validating — **the doc-gate** (wraps and absorbs the existing `lint-docs.sh` /
`lint-mermaid.sh`):
- `living-docs check [paths…]` — OKF frontmatter schema, one-home/indexed invariant,
  supersede-chain integrity, Mermaid validity, broken links. Exit code is the gate
  verdict (0 = conform).

**The boundary is the whole point.** The CLI never writes Context / Decision /
Consequences prose, never chooses the epistemic type, never resolves which alternative
wins. Those stay with the authoring model. The CLI is deterministic-only by construction —
there is no LLM inside it.

This decision is **independent of, and a precondition for**, any later move to render doc
prose with a cheaper model: `living-docs check` is the deterministic verifier that makes
such a split safe (structural drift becomes a failed exit code, and the authoring model's
fidelity check collapses to "run `check`, read the code" instead of re-reading the doc).
Whether to build that cheaper-render split is a separate, later decision that must rest on
measured authoring cost, not on plausibility — it is explicitly out of scope here.

## Consequences

**Easier / gained:**
- Mechanical steps become deterministic and free: correct numbering, never-stale indexes,
  always-bidirectional supersede links, schema-conformant frontmatter.
- A single `check` gate a CI job, a review step, or a future cheaper authoring model can
  all consume as a hard signal.
- The `new`/`index`/`supersede` seam is the natural place to enforce the "indexed or it
  doesn't exist" invariant instead of trusting an agent to remember it.

**Harder / accepted trade-offs:**
- A CLI is code to maintain, test, and version alongside the skill (a script surface the
  skill did not previously own beyond the two lint scripts).
- Templates and the CLI's frontmatter-fill logic are now coupled: a template frontmatter
  change must stay in lock-step with `new`. Mitigated by making `new`'s output pass
  `check` in the test suite (the fitness function below).
- The CLI does **not** by itself reduce authoring cost — the cost lives in rationale prose
  output, which the CLI cannot write. Its cost value is indirect: it is the safety
  precondition for a later cheap-render split, and it removes cheap-but-nonzero mechanical
  tokens from the authoring model.

**Follow-ups:**
- A separate ADR (deferred, evidence-gated) may propose rendering doc prose with a cheaper
  model, using `living-docs check` as the fidelity gate — only after doc-authoring token
  cost is measured.
- ~~Decide the CLI's host language/runtime (POSIX shell to extend the existing scripts vs.
  a richer runtime) — an implementation issue, not this decision.~~ Resolved: Rust (see
  `cli/`); a self-contained, cross-compiled binary distributed via prebuilt release assets.

## Verification

**Implementation impact:** new CLI entry point under `skills/living-docs/scripts/`
(absorbing `lint-docs.sh` + `lint-mermaid.sh` behind `living-docs check`); the repo's own
`docs/adr/` trail (this file + `index.md`) becomes its first consumer.

**Verification criteria:**
- `living-docs new adr "x"` produces a file that passes `living-docs check` with exit 0
  (fitness function: a test that scaffolds then checks).
- `living-docs index` is idempotent — running it twice against the same corpus yields no
  diff (fitness function: generate-twice, `diff` must be empty).
- `living-docs supersede A B` leaves both records mutually linked and both still pass
  `check` (supersede-chain integrity is part of `check`).
- `living-docs check` runs in CI over `docs/` and fails the build on any non-conformance
  (fitness function: CI gate; this is the doc-gate the decision names).
