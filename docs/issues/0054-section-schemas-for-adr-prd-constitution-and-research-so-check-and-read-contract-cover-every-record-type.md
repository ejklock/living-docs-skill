---
type: Issue
title: Section schemas for ADR, PRD, constitution and research so check and read --contract cover every record type
description: "Fills the remaining section-schema rows ADR 0064 left empty: required and optional sections with their tier for ADR, PRD, constitution and research."
status: closed
timestamp: 2026-10-05T23:16:04Z
---

## 0054. Section schemas for ADR, PRD, constitution and research so check and read --contract cover every record type

Implements the follow-ups of [ADR 0064](/adr/0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md). [Issue 0052](/issues/0052-section-schema-in-the-registry-check-requires-scope-and-acceptance-on-live-issues-and-read-gains-a-contract-level.md) populated only the issue row, so a live ADR with no Consequences still passes `check`, and `read --contract` shows every other type whole.

### Scope

The registry rows, each section matched by heading name:

| Type | Required, contract | Optional, contract | Optional, detail |
|---|---|---|---|
| ADR | Context, Decision, Consequences | Verification | References |
| PRD | Problem / Motivation, Non-goals | Goals, Requirements, Acceptance criteria, Success metrics | Open questions, Related |
| Constitution | Product, Scope Boundaries, Non-negotiables | Data Model / Schema Foundation | Amendment Log |
| Research | none | Question, Findings, Implications | Method, Open Questions, References |

Each type's guide topic names its required sections in one line. Kept unchanged: the architecture view row (a diagram with no prose sections), the issue row, and the check and `read --contract` code.

### Decision

- One issue for the four rows instead of one per type, as ADR 0064's follow-up line suggested: the diff is one registry file plus guide lines, and the rows ship together. Option not taken: four issues with identical plans.
- Research requires nothing. Its format belongs to the `research-artifacts` skill, and Accepted research is live by the registry, so a required section would fail research written under that skill's freer structure (this repo's research 0001). Option not taken: requiring Question and Findings.
- ADR `Verification` is contract, not detail: it binds the decision to a check, which a reader orienting on the decision needs.
- The ADR rule reaches further than the issue rule did: 14 tests across 8 files carried prose-only live ADR or constitution fixtures and went red. A downstream bundle with prose-only Proposed or Accepted ADRs fails `check` after upgrading; the release notes name the three headings and `check --changed-files` as the migration path. Option not taken: making ADR sections optional, which would leave a live ADR with no Consequences unchecked.

### Acceptance

- A live ADR without `## Consequences` fails `check`, naming the section; a Deprecated one passes.
- A Draft PRD without `## Non-goals` fails `check`.
- The constitution without `## Non-negotiables` fails `check`.
- A research note with none of its template sections passes `check`.
- `read --contract` omits an ADR's `References` and a research note's `Method`.
- The template fitness test passes for every row, and `living-docs check` over `docs/` stays OK.

### Plan

1. Registry rows and their unit tests; fitness test green.
2. Integration cases through the binary; guide lines; dogfood over `docs/`.

### Outcome

Closed: the four rows ship in `living-docs-core/src/doc_type/sections.rs` beside the issue row, and the suite went from 636 to 650 tests. Fourteen existing tests needed their live ADR or constitution fixtures to carry the required headings, which is the same migration a downstream bundle faces. A PRD test for a missing `Problem / Motivation` rides with [issue 0053](/issues/0053-section-heading-parser-counts-indented-code-headings-and-always-drops-the-first-heading-as-the-title.md).
