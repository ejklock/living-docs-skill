---
type: ADR
title: Web UX follows the three-pane doc-site archetype with a search-first Cmd+K palette
description: The web front adopts the doc-site three-pane layout (type-grouped nav tree, rendered record, metadata/relations panel) with FTS5-backed search-first navigation via a Cmd+K palette — not a Notion-style block editor UX.
status: Accepted
tags: [frontend, ux, web]
timestamp: 2026-07-19T02:16:09Z
---

# 0015. Web UX follows the three-pane doc-site archetype with a search-first Cmd+K palette

## Context

Issue [0003](/issues/0003-web-read-only.md) delivers a
read-only, server-rendered axum + maud view over the SQLite/FTS5 read-model (ADR 0006:
no in-browser editing — authoring stays in the CLI + model). With the minimal
search-page + record-page slices defined, the open fork was which UX archetype the web
front grows into.

Candidates considered:

- **Notion-style database views.** Notion's core value is its block *editor*, which
  ADR 0006 forbids here; what remains (filterable tables, clean typography) does not
  justify the heavier client-side surface it implies.
- **Decision-timeline home (log4brains).** Strong for the ADR supersede trail, weak for
  general browsing of PRDs, BDRs, issues, and research notes.
- **Doc-site three-pane (mkdocs-material / GitBook)** with **search-first navigation
  (Linear-style Cmd+K palette)**. Matches a read-only corpus grouped by type, keeps the
  FTS5 read-model — the system's findability core — as the primary entry point, and is
  achievable server-rendered.

The determinism boundary and the maud/server-rendered stack (plan for issue 0003) bound
the choice: the archetype must not require a heavy client framework.

## Decision

We will build the web UX as a **three-pane doc-site**: a left navigation tree grouped by
record type (ADR, BDR, PRD, issue, research), a center pane rendering the record body,
and a right metadata panel showing status, supersede chain, and cross-links
(log4brains-style lifecycle presentation). Primary navigation is **search-first**: a
Cmd+K palette querying the FTS5 read-model with ranked results. Rendering stays
server-side (axum + maud); client-side JavaScript is limited to progressive enhancement
of the palette (vanilla JS or htmx), never a client framework.

## Consequences

**Easier / gained:**
- The FTS5 read-model is the front door — search quality directly shapes UX value.
- The supersede/status lifecycle (the corpus's distinguishing trait) gets a first-class
  visual surface in the metadata panel.
- Newcomers get browsable orientation (nav tree by type) without knowing what to search.
- Stays within the locked stack: no client framework, no build-tooling for the front end.

**Harder / accepted trade-offs:**
- No Notion-style database/board views; filtering is limited to the nav tree and search.
- The Cmd+K palette needs a small JS surface, which must stay progressive-enhancement
  (the plain GET /?q= form remains the no-JS fallback).
- Three panes require layout/CSS work beyond the minimal slices of issue 0003.

**Follow-ups:**
- A post-0003 issue for the three-pane shell: nav tree, metadata panel, palette
  (issue 0003's search + record pages ship first as the walking surface).
- The metadata panel needs supersede/status/links exposed by the read-model; extend the
  db-store projection if the current schema does not carry them.

## Verification

**Implementation impact:** `web/src/main.rs`, `web/src/views.rs`, `db-store` (relation
metadata in the projection), `tests/browser/`.

**Verification criteria:**
- The record page renders nav tree, body, and metadata panel; the metadata panel shows
  status and the supersede chain for a superseded record (browser spec).
- Disabling JavaScript still yields working search via the GET /?q= form (browser spec
  with JS disabled).
- The router remains GET-only (ADR 0006 fitness function stays green).
