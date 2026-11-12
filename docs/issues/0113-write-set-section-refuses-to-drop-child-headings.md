---
type: Issue
title: write set section refuses to drop child headings
description: "Implements ADR 0080: replacing a section with child headings the new text does not carry exits 2 and writes nothing."
status: open
timestamp: 2026-11-12T16:14:04Z
---

## 0113. write set section refuses to drop child headings

Implements [ADR 0080](/adr/0080-write-set-section-refuses-to-drop-child-headings-the-new-text-does-not-carry.md): `write set <ref> section <heading>` refuses with exit 2, writing nothing, when the section holds child headings the new text does not carry, and names each missing child.

### Scope

- `living-docs-core/src/sections.rs` and `living-docs-core/src/commands/set.rs` (`apply_section`): find the section's child headings, outside code blocks, and refuse when one is missing from the new text at the same level and text.
- The `set` help for the `section` key and the procedure guide topic.
- Kept: replacement of a section without children, the append path, the heading matcher.

### Acceptance

- A section with one child heading and new text without it: exit 2, the record byte-identical, the error naming the child.
- The same with the child carried in the new text: exit 0, the child present once.
- A section without children: replaced as today; a missing section: appended as today.
- A child carried at another level, or with other text, counts as missing.
- A heading inside a fenced or indented code block is not a child.
- The MCP `write` tool's `set` operation returns the same refusal.
