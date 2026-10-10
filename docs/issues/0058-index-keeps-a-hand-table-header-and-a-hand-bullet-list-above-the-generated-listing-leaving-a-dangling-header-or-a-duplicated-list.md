---
type: Issue
title: index keeps a hand table header and a hand bullet list above the generated listing, leaving a dangling header or a duplicated list
description: "A hand-maintained index migrates into a broken file: its table header survives alone, or its hand bullets are listed twice."
status: open
timestamp: 2026-10-10T19:16:42Z
---

## 0058. index keeps a hand table header and a hand bullet list above the generated listing, leaving a dangling header or a duplicated list

Found adopting 0.21.0 in a downstream bundle (relent). `index` keeps everything above the first boundary line byte-for-byte and regenerates the rest. `is_boundary_line` in `living-docs-core/src/commands/index.rs` accepts a `## ` heading, a `* [` bullet, and a table row whose first cell is `#` or a record link. It does not accept a `- [` bullet or a table header whose first cell is any other word.

Reproduction 1 (dangling header): `docs/adr/index.md` holds `# ADRs`, a blank line, `| ADR | Decision |`, `|---|---|`, `| [0001](x.md) | y |`. After `living-docs index adr` the file keeps `| ADR | Decision |` and `|---|---|` and then `## Active`: a table header with no rows.

Reproduction 2 (duplicate list): `docs/research/index.md` holds an intro and `- [Foo](0001-foo.md) - hand summary`. After `living-docs index research` the hand bullet stays in the preamble and `* [0001 — Foo](0001-foo.md) - Draft` is appended: the listing is duplicated. In relent this duplicated all 18 research entries.

Both are silent: `check` passes and a second `index` run is a no-op, so the damage is only visible by reading the file.

### Scope

Teach the boundary detection what a hand listing looks like, so a migrated index regenerates cleanly. Kept: preamble preservation for genuine framing prose, and idempotence.

### Acceptance

- A table header row (any first cell) followed by a separator row and record rows is treated as part of the hand listing, so it does not survive above `## Active`.
- A `- [` bullet is a boundary like `* [`, so a hand bullet list is replaced, not duplicated.
- Tests carry both reproductions as fixtures and assert the generated file has no header without rows and no repeated entry.
- A second `index` run is still a no-op, and the existing preamble tests stay green.
- `living-docs check docs` stays OK.
