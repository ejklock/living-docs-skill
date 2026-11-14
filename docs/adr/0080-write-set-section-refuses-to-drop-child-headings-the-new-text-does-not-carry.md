---
type: ADR
title: write set section refuses to drop child headings the new text does not carry
description: Replacing a body section that holds child headings is refused with exit 2 unless the new text carries every child, so a section rewrite never deletes a subsection the caller did not see.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-12T16:13:09Z
---

# 0080. write set section refuses to drop child headings the new text does not carry

## Context

`write set <ref> section "<heading>" --from PATH|-`, from [ADR 0066](/adr/0066-record-prose-enters-through-a-file-or-stdin-new-body-and-set-from-replace-argv-for-body-and-field-values.md), replaces the text under the first body section whose heading matches and keeps the heading line. "The text under the heading" runs to the next heading of the same or a higher level, so it includes every child heading and its text.

The verb therefore deletes structure the caller never mentioned. Replacing an issue's `### Scope` that holds a `#### Out of scope` child with one new paragraph removes the child and its text, and the command exits 0 with nothing on stderr (reproduced with the current build). The caller is usually an agent that read the section's first lines, not its whole subtree, and the loss surfaces only in a later diff review, if at all. ADR 0066 states the tool only replaces the text under one heading; it never said the replacement may silently drop headings the author wrote.

## Decision

We will refuse the replacement, with exit 2 and nothing written, when the existing section holds child headings and the new text does not carry every one of them. A child is carried when the new text holds a heading with the same level and the same text, matched the way the section schema matches headings. The error names each missing child and says to include it in the new text or remove it on purpose by editing the body. A section with no child headings, and an append to a record without the section, behave as today.

Rejected alternatives:

- **Warn on stderr and proceed.** The text is already gone when the warning prints, and an agent reading only the exit code never sees it.
- **Keep the replacement as it is.** It leaves a destructive default on the verb agents use most to edit a body.
- **Replace only up to the first child heading.** It keeps the children but changes what "the section" means for every caller and leaves no way to rewrite a section whole.

## Consequences

**Easier / gained:**
- A section rewrite can no longer delete a subsection the caller did not see.
- The refusal names the missing children, so the fix is one edit to the new text.

**Harder / accepted trade-offs:**
- Removing a child heading on purpose needs a body edit instead of `set section`.
- The MCP `write` tool's `set` operation inherits the refusal, as it shares the core function.

**Follow-ups:**
- An issue implementing the refusal, with the deciding cases below as its acceptance.

## Verification

**Implementation impact:** `living-docs-core/src/commands/set.rs` (`apply_section`), `living-docs-core/src/sections.rs`, their tests, the `set` help text and the procedure guide topic.

**Verification criteria:**
- A section with one child heading, new text without it: exit 2, the record byte-identical, the error naming the child.
- The same with the child carried in the new text: exit 0, the child present once.
- A section with no children: replaced as today. A missing section: appended as today.
- A child carried at another level, or with other text, counts as missing.
- A heading inside a fenced or indented code block is not a child.
