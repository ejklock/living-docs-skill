---
type: Issue
title: The liveness declaration scan walks links with its own parser instead of link_destinations
description: Give link_destinations the source range of each link so the liveness Implements scan reuses it instead of a second parser walk.
status: open
timestamp: 2026-11-12T18:49:10Z
---

## 0117. The liveness declaration scan walks links with its own parser instead of link_destinations

The liveness check walks a record's markdown with its own `pulldown_cmark::Parser::new(contents).into_offset_iter()` (`living-docs-core/src/check/liveness.rs`, the `Implements` declaration scan) instead of reusing `link_destinations` in `living-docs-core/src/check/links.rs`, because `link_destinations` returns bare destinations without the source offset the declaration scan needs to read the text before each link. Two walks of the same parser with the same options recognize the same links today; they can drift if one gains parser options or link kinds the other lacks.

### Scope

- `link_destinations` (or a sibling in `links.rs`) yields each destination with its source range; the liveness scan consumes it and drops its own parser walk.
- Kept: link recognition, the parser options, and every `check` verdict and advisory.

### Acceptance

- `git grep -n 'Parser::new' living-docs-core/src/check` lists only `links.rs`.
- `cargo test --workspace` passes with no test changed except those that pin the new return shape.

### Decision

- The scan keeps counting links only: `link_starts` reports each destination with its kind (link or image) and liveness skips images, so `Implements ![x](/adr/...)` stays uncounted as before and ADR 0079 holds. The first review found that a shared link-or-image filter made it count. Not taken: tracking that behavior change in a separate issue; treating an image as a declaration (owner, 2026-11-12).
