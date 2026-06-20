# BDR Conventions

A Behavior Decision Record specifies **what a system must observably do** — inputs, outputs, side effects, and the scenarios that constitute correct operation. A BDR answers *what and observable*; an ADR answers *how is it structured and why*.

> **Provenance — instrumentalization, not invention.** A BDR's core — observable behavior captured as **Given/When/Then** scenarios written to convert verbatim into the test suite — is **Specification by Example / BDD** (ADZIC, *Specification by Example*, 2011; NORTH, *Introducing BDD*, 2006), not original here. "Behavior Decision Record" as a *named record type* is a recent third-party coinage (ZANZAL, 2026), itself an ADR-style wrapper over Specification by Example; treat it as such, not as an industry standard or a repo invention. Our only addition is binding the scenarios to the pipeline (verbatim → regression suite, mutation-gated). Full citations: `../../../references/prior-art-landscape.md`.

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
   are consumed by `implementation-review`. A standalone, addressable record (`/bdr/NNNN`) is
   what code, tests, and review all point at — a buried PRD subsection is not.

This is a packaging choice over Specification by Example, not a new method — see Provenance below.

## Format

Each BDR is an **OKF concept** (`type: BDR`) — see the `okf-knowledge-format` skill. `status` (`Draft` | `Accepted` | `Implemented` | `Superseded`) lives in the frontmatter, not a body line. See `templates/bdr.md`. Core sections:

- **Context** — why this behavior needs to be specified, and the PRD/ADR/issue that motivated it.
- **Behavior diagram** — a Mermaid flowchart or sequence diagram of the full observable flow.
- **Textual description** — prose of the same behavior, written from the outside (inputs, outputs, side effects, error paths).
- **Scenarios** — numbered Given/When/Then statements written to convert verbatim into the project's behavioral regression suite.
- **Related** — links to the PRD, ADR, and issues that this BDR serves.

## Rules

1. **Every BDR has three required elements: a Mermaid diagram AND a textual description AND numbered Given/When/Then scenarios.** All three are mandatory; no section may be left empty or removed.
2. **Scenarios are written to convert verbatim into the project's behavioral regression suite.** Use the exact wording a test author would need. Avoid implementation detail; describe what an external observer would verify.
3. **BDRs answer *what*; ADRs answer *how*.** If the record is about structure, technology choice, or rationale — write an ADR. If it is about observable behavior a user or system can verify — write a BDR.
4. **BDRs are spawned (or amended) by a PRD that changes expected behavior.** Every BDR must link to the PRD that produced it. A BDR written without a parent PRD is a sign the PRD is missing.
5. **Diagrams are Mermaid only.** No ASCII art, no image attachments.
6. **Numbered sequentially:** `docs/bdr/NNNN-slug.md`. Index in `docs/bdr/index.md` (OKF reserved listing, no frontmatter) with a one-line summary and a link per record.
7. **Append-only once accepted.** After a BDR is accepted, changes to specified behavior are recorded as dated Amendment sections appended to the file, or the BDR is superseded by a new one (frontmatter `status: Superseded`, `superseded_by: NNNN`). Silent in-place edits are not allowed.

## Anti-patterns

- A BDR with no Mermaid diagram. If the behavior cannot be drawn, the behavior is not yet understood.
- Scenarios that describe implementation steps ("Given the service calls the database..."). Scenarios describe observable outcomes, not internal mechanics.
- A BDR written without a parent PRD. Trace back to the PRD and link it before marking the BDR Accepted.
- Editing accepted scenarios in place when behavior changes — amend or supersede so the history of what was agreed survives.
