---
type: Issue
title: Give the generated HTML a sidebar navigation and a reading style
description: manage html renders a sticky sidebar of types and statuses beside the running page of records, with light and dark themes, status badges and a print style, still one deterministic offline file.
status: open
timestamp: 2026-11-27T19:32:22Z
---

## 0164. Give the generated HTML a sidebar navigation and a reading style

## Context

`living-docs manage html` (ADR 0090) writes one self-contained page: a heading, a filter box and every record stacked in groups by type and status (`living-docs-core/src/html/page.rs`). The owner asked on 2026-11-27 for a side menu and a better style.

## Decision

- Layout: a sticky sidebar holding the filter and a tree of types, each with its statuses and record counts, linking to anchors in a running page of records on the right; the sidebar marks the group in view (owner, 2026-11-27). Not taken: one record at a time behind hash navigation; three panes (types, list, record).
- Style: light and dark themes from `prefers-color-scheme`, a readable line length, coloured status badges, metadata as chips, backlinks and the supersede chain in a record footer, and a print stylesheet that hides the sidebar.
- Zoom: clicking a diagram or an image opens a full-screen overlay with wheel zoom, drag to pan and Escape to close, in a small inline script (owner, 2026-11-27). Not taken: React, which needs its runtime inlined in every page or a Node build and would supersede ADR 0090's no-template-engine, small-script limit.
- ADR 0090's limits stay: one file, byte-identical output for the same bundle, no server, only the small inline filter script plus Mermaid.

## Acceptance

- The page renders the sidebar tree with counts that equal the records rendered, and each entry links to an anchor that exists.
- Output stays byte-identical for an unchanged bundle; the page works offline from `file://`.
- Clicking a Mermaid diagram or an image opens the zoom overlay; wheel, drag and Escape work offline.
- The filter still hides non-matching records and empties groups, and the sidebar counts follow it or the sidebar marks hidden groups.

## Scope

In: `living-docs-core/src/html/` markup, style and script. Out: a server, search index or multi-page site.
