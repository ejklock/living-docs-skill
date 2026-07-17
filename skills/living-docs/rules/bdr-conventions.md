# BDR Conventions

A Behavior Decision Record specifies **what a system must observably do** — inputs, outputs, side effects, and the scenarios that constitute correct operation. A BDR answers *what and observable*; an ADR answers *how is it structured and why*.

> **Provenance — instrumentalization, not invention.** A BDR's core — observable behavior captured as **Given/When/Then** scenarios written to convert verbatim into the test suite — is **Specification by Example / BDD** (ADZIC, *Specification by Example*, 2011; NORTH, *Introducing BDD*, 2006), not original here. "Behavior Decision Record" as a *named record type* is a recent third-party coinage (ZANZAL, 2026), itself an ADR-style wrapper over Specification by Example; treat it as such, not as an industry standard or a repo invention. Our only addition is binding the scenarios to the test suite (verbatim → regression suite). Full citations: `../../../references/prior-art-landscape.md`.

## Why a separate record, not a section of the ADR/PRD

A fair challenge: if a BDR is "Given/When/Then in an ADR-style file", why not make it a
mandatory *Scenarios* section of the ADR or PRD instead of a new document type with its own
numbering and directory? The split earns its keep on three counts — if none of them hold for
your project, **do** fold scenarios into the PRD and skip the genre:

1. **Different lifecycle.** An ADR is settled once and superseded as a whole. Behavior accretes:
   one capability grows scenarios over many changes, and individual scenarios are amended or
   superseded independently of the structural decision. Bundling them forces a structural
   supersede every time an edge case is added.
2. **Different cardinality.** One PRD spawns many behaviors, and one behavior may serve several
   PRDs/ADRs. A separate record gives each behavior one home to link to (invariant 2) instead of
   duplicating scenarios across the docs that touch them.
3. **It is the test contract.** The scenarios convert *verbatim* into the regression suite and
   are consumed by whatever review step verifies the change. A standalone, addressable record (`/bdr/NNNN`) is
   what code, tests, and review all point at — a buried PRD subsection is not.

This is a packaging choice over Specification by Example, not a new method — see Provenance below.

## Format

Each BDR is an **OKF concept** (`type: BDR`) — see the `okf-knowledge-format` skill. `status` (`Draft` | `Accepted` | `Implemented` | `Superseded`) lives in the frontmatter, not a body line. See `templates/bdr.md`. Core sections:

- **Context** — why this behavior needs to be specified, and the PRD/ADR/issue that motivated it.
- **Behavior diagram** — a Mermaid flowchart or sequence diagram of the full observable flow.
- **Textual description** — prose of the same behavior, written from the outside (inputs, outputs, side effects, error paths).
- **Contract / Surface** — the observable surface that realizes the behavior: the public function/method/class signatures a caller invokes, and (for agentic systems) each agent tool's name + input/output (function-calling) schema. Signatures and schemas only — the internal call graph stays in the ADR and the code.
- **Scenarios** — numbered Given/When/Then statements written to convert verbatim into the project's behavioral regression suite.
- **Test Design** — *how* each behavior is tested: the test matrix (happy / equivalence / boundary / error / property) derived from the scenarios, each row naming what it PROVES and at which level. This is the **single home** for the "how to test" of the behavior; an execution issue links here, never copies the matrix.
- **Related** — links to the PRD, ADR(s) — including any test-strategy ADR — and issues that this BDR serves.

## Rules

1. **Every BDR has five required elements: a Mermaid diagram AND a textual description AND a Contract section AND numbered Given/When/Then scenarios AND a Test Design matrix.** All five are mandatory; no section may be left empty or removed. The Test Design matrix is derived from the scenarios (one G/W/T is one example, not the spec) and every row names what it proves.
2. **Scenarios are written to convert verbatim into the project's behavioral regression suite.** Use the exact wording a test author would need. Avoid implementation detail; describe what an external observer would verify.
2a. **The Contract section names the observable surface, never the internal call graph.** List the public signatures a caller invokes and, for an agentic system, the agent tools' function-calling schemas (name + input/output) — the contracts a caller or agent sees. Do **not** list private helpers, internal call order, or step-by-step wiring; that is the ADR's *how* and the code's business. Map each entry to the scenario/outcome it realizes. Omit the Public-API or Agent-tools table when it does not apply — but never omit the section.
3. **BDRs answer *what*; ADRs answer *how*.** If the record is about structure, technology choice, or rationale — write an ADR. If it is about observable behavior a user or system can verify — write a BDR.
3a. **The test of a behavior has one home: the BDR's Test Design.** *What* is tested = the scenarios; *how* it is tested = the Test Design matrix derived from them (both in the BDR). The *why* of a test-strategy choice (a non-default level/technique, or a deviation from your project's standing test bar — a decision with a rejected alternative) is a **test-strategy ADR** (an ordinary ADR carrying `tags: [testing]`, see `rules/adr-conventions.md`), linked from the BDR — not a separate record type, and not duplicated in the matrix. There is deliberately **no "Test Decision Record"**: the need splits cleanly across BDR (what/how) + test-strategy ADR (why) + your CI gate (enforce); the field's direction is "**Any** Decision Record" (one template absorbs domain decisions, MADR 3.0), and "TDR" already names "Technical Debt Record".
4. **BDRs are spawned (or amended) by a PRD that changes expected behavior.** Every BDR must link to the PRD that produced it. A BDR written without a parent PRD is a sign the PRD is missing.
5. **Diagrams are Mermaid only.** No ASCII art, no image attachments.
6. **Numbered sequentially:** `docs/bdr/NNNN-slug.md`. Index in `docs/bdr/index.md` (OKF reserved listing, no frontmatter) with a one-line summary and a link per record.
7. **Append-only once accepted.** After a BDR is accepted, changes to specified behavior are recorded as dated Amendment sections appended to the file, or the BDR is superseded by a new one (frontmatter `status: Superseded`, `superseded_by: NNNN`). Silent in-place edits are not allowed.

## Anti-patterns

- A BDR with no Mermaid diagram. If the behavior cannot be drawn, the behavior is not yet understood.
- Scenarios that describe implementation steps ("Given the service calls the database..."). Scenarios describe observable outcomes, not internal mechanics.
- A BDR written without a parent PRD. Trace back to the PRD and link it before marking the BDR Accepted.
- A Contract section that lists internal/private calls or step-by-step wiring instead of the public surface — that is the ADR's *how*, not the BDR's observable contract.
- Editing accepted scenarios in place when behavior changes — amend or supersede so the history of what was agreed survives.
