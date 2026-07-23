---
type: ADR
title: BDR carries a required Contract section (public API + agent tool schemas)
description: Every BDR must name the observable contract that realizes the behavior — public function/method/class signatures and, for agentic systems, agent tool schemas — while keeping the internal call graph out.
status: Accepted
tags: [bdr, documentation, methodology]
timestamp: 2026-07-17T21:01:45Z
---

# 0008. BDR carries a required Contract section (public API + agent tool schemas)

## Context

A BDR specifies observable behavior — Given/When/Then scenarios that convert verbatim into
the regression suite. Its founding boundary is *what/observable, not how/internal*: rule 3
("BDRs answer *what*; ADRs answer *how*"), plus rule 2 and an anti-pattern that forbid
scenarios describing internal mechanics.

That boundary leaves a real gap. Nothing in the BDR binds the specified behavior to the
concrete surface that realizes it. A reader knows what the system must do, but not which
functions/methods/classes a caller invokes to get it — and, for an agentic system, not which
agent tools (function-calling) expose or drive the behavior. The behavior→surface link has no
home today: putting it in an ADR scatters it across decisions; putting it in an issue ties it
to one execution slice instead of the behavior's one home.

The naive fix — a full internal call plan in the BDR — reintroduces exactly the *how* the BDR
exists to exclude (private helpers, call order), collapsing the what/how split that justifies a
separate record in the first place.

There is an observable middle ground. A public signature and an agent tool's input/output
schema are both **contracts a caller or agent sees** — they are observable, not internal
mechanics. The internal call graph (which private method calls which, in what order) is not.

## Decision

We will add a **required `## Contract` section to every BDR** that names the *observable*
surface realizing the behavior, and only that surface:

- **Public API** — the functions/methods/classes a caller invokes, with signatures. Each entry
  maps to the scenario/outcome it realizes.
- **Agent tools** (agentic systems only) — the function-calling surface: each tool's name, input
  schema, and output/effect. Each maps to the scenario/outcome it realizes.

The section carries signatures and schemas only. Internal call sequences, private helpers, and
ordering stay out — those remain the ADR's *how* and the code's business. A BDR omits whichever
of the two tables does not apply, but may never omit the section.

Enforcement is at the **convention level**: the Contract section joins the BDR's existing
required elements (Mermaid diagram, textual description, scenarios, Test Design matrix) in
`rules/bdr-conventions.md` rule 1 and `templates/bdr.md`. Like those four, it is honored by
authors and the review step — not yet by `living-docs check`, which validates structural
invariants, not per-type body sections.

## Consequences

**Easier / gained:**
- The behavior→surface link finally has one home: a reader goes from a scenario to the exact
  public signature or agent tool that satisfies it.
- Agentic systems get a first-class place to specify the tool-calling contract next to the
  behavior it serves, instead of only a free-floating tool-calling diagram in the architecture
  views.
- The what/how boundary survives: the section admits only observable contracts
  (signatures/schemas), never the internal call graph.

**Harder / accepted trade-offs:**
- Every BDR now carries a fifth required section; a behavior with a trivial surface still pays
  it (mitigated by "omit the table that doesn't apply" — but never the section).
- The signature/schema written in the BDR can drift from the code, like any contract kept
  outside the compiler. Convention-only enforcement makes the review step the backstop until a
  checker exists.
- The line between "public contract" and "internal mechanics" is a judgement call at the
  margins (a stable-but-internal seam); authors resolve it toward observable-only.

**Follow-ups:**
- A future `living-docs check` rule could enforce presence of all five required BDR sections,
  turning the convention into a mechanical invariant.
- Where an architecture tool-calling diagram exists, link the Contract's agent-tool entries to
  it so the two surfaces stay consistent.

## Verification

**Implementation impact:** `skills/living-docs/rules/bdr-conventions.md` (Format list, rule 1, a
new boundary rule, Anti-patterns) and `skills/living-docs/templates/bdr.md` (a new `## Contract`
section).

**Verification criteria:**
- `rules/bdr-conventions.md` lists Contract/Surface among the required BDR elements and states
  the observable-only boundary (public signatures + agent tool schemas; no internal call graph).
- `templates/bdr.md` carries a `## Contract` section with a Public API table and an Agent tools
  table, plus the instruction to omit the non-applicable table but never the section.
- `living-docs check docs` stays green over `docs/` with this ADR indexed.
