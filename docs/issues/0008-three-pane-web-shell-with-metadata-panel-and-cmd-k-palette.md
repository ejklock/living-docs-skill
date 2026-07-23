---
type: Issue
title: Three-pane web shell with metadata panel and Cmd+K palette
description: Implement the ADR 0015 web UX — type-grouped nav tree, record body pane, status/supersede metadata panel, and a progressive-enhancement Cmd+K search palette.
status: done
labels: [web, ux]
blocked_by: []
timestamp: 2026-07-19T02:22:09Z
---

## Three-pane web shell with metadata panel and Cmd+K palette

Implements [ADR 0015](/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md)
on top of the read-only web front delivered by
[issue 0003](/issues/0003-web-read-only.md). The web view grows from two bare pages
(search, record) into the three-pane doc-site archetype: left nav tree grouped by record
type, center record body, right metadata panel (status badge, supersede chain,
cross-links), with a search-first Cmd+K palette over the FTS5 read-model.

### Scope

- db-store read-model extensions: `status` column extracted from frontmatter, a
  listing-by-type query for the nav tree, and a record-metadata getter (status,
  supersedes/superseded_by via the existing relations table, tags).
- Three-pane server-rendered layout (axum + maud + one static CSS route) applied to the
  search and record pages.
- Metadata panel on the record page: status badge, navigable supersede chain, tags.
- Cmd+K palette as progressive enhancement (small vanilla JS asset + an HTML-fragment
  results endpoint); the plain `GET /?q=` form remains the no-JS fallback.
- KEPT: read-only GET-only router (ADR 0006 fitness), maud auto-escaping, async
  db-store calls in handlers (never the sync port — lesson 3671).

### Acceptance

- The record page renders nav tree, body, and metadata panel; a superseded record shows
  its status and a navigable supersede chain (browser spec).
- The nav tree groups records by type and marks the current record (browser spec).
- With JavaScript disabled, search via `GET /?q=` still works end-to-end (browser spec).
- The Cmd+K palette opens, queries FTS5, and navigates to a picked record (browser spec).
- A mutating HTTP method still returns 405 (read-only fitness stays green).
- `living-docs check` passes; `cargo test --workspace` green.

### Plan

Four vertical slices, each demoable:

1. **S1 — read-model:** `status` column + extraction in sync, `records_by_type` listing,
   `record_meta` getter (status, supersede chain, tags). Unit-tested in db-store.
2. **S2 — shell:** three-pane layout + CSS route; nav tree from `records_by_type` on
   search and record pages.
3. **S3 — metadata panel:** status badge, supersede chain, tags on the record page.
4. **S4 — palette:** Cmd+K overlay (vanilla JS, `defer`), HTML-fragment endpoint
   `GET /palette?q=`, JS-disabled fallback spec.
