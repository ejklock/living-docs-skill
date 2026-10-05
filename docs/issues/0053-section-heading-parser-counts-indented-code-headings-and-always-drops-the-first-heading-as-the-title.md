---
type: Issue
title: Section heading parser counts indented-code headings and always drops the first heading as the title
description: heading_names, used by the required-section check, treats a heading inside a 4-space indented code block as a section and drops the first heading even when a record has no title heading.
status: open
timestamp: 2026-10-05T22:11:15Z
---

## 0053. Section heading parser counts indented-code headings and always drops the first heading as the title

Follow-up to [issue 0052](/issues/0052-section-schema-in-the-registry-check-requires-scope-and-acceptance-on-live-issues-and-read-gains-a-contract-level.md), from its slice A review. `heading_names` in `living-docs-core/src/sections.rs` feeds the required-section check of [ADR 0064](/adr/0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md), and it has two gaps:

- **Indented code.** A line indented four or more spaces is code in CommonMark, but the parser trims it and reads `    ### Acceptance` as a heading. A live issue could satisfy the check with a code sample.
- **First heading as the title.** The first heading is always dropped as the title. A record with no title heading that opens with `### Scope` loses that section and fails the check as if Scope were missing. `read --contract` uses its own splitter and is not affected.

### Scope

- A heading indented four or more spaces outside a list is code, never a section, for both `heading_names` and `contract_body`.
- The title is dropped only when the first heading is the record's title, judged against its `title` frontmatter or its level, never by position alone.

Kept unchanged: the fence rules, the exact case-insensitive name match, and setext headings staying unrecognized. Setext reaches only levels 1 and 2, and declared sections sit below the title.

### Acceptance

- A live issue whose only `Acceptance` heading is indented four spaces fails `check` for the missing section.
- A live issue with no title heading whose body opens with `### Scope` and also has `### Acceptance` passes `check`.
- The existing section tests and the `read --contract` tests pass unchanged.
