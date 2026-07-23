---
type: Issue
title: Read-only web view — axum search + record page over the read-model
description: Serve a read-only, server-rendered axum web view that searches the read-model and renders a record, reusing living-docs-core — the browser-observable surface of findability.
status: done
labels: [slice, web, findability]
blocked_by: [2]
timestamp: 2026-07-16T00:00:00Z
---

## Read-only web view — axum over the read-model

Implements [ADR 0006](/adr/0006-web-read-only-axum.md), reusing
[ADR 0002](/adr/0002-hexagonal-core-workspace.md) core. **Slice 2** — findability you can
open in a browser.

### Objective link

Constitution (findability) → ADR 0006 (read-only axum reusing core) → this slice makes the
slice-1 search browsable.

### Context manifest

- Read: `living-docs-core` `SearchIndex` / `DocStore` ports, the `db-store` read-model (slice
  0002), [ADR 0006](/adr/0006-web-read-only-axum.md).
- Seams touched: new `web` crate (axum + HTML templates); a search route and a record-view
  route; no write routes.
- Pattern: server-rendered, read-only; reuses the core query — no second record model.

### Scope

An axum server with two routes: `GET /` (search box + results for `?q=`) and
`GET /record/...` (renders a record's body). Reads the read-model via core ports. Read-only:
no create/edit/delete endpoints.

### Vertical Demo

- **Given** a synced read-model, **When** I open `/`, type `supersede`, and submit,
  **Then** ADR 0003 shows in the results; clicking it renders its body.
- **Given** a query with no match, **When** I search `zzzznomatch`, **Then** the page shows
  an empty-state message (unhappy path), not an error.

### Acceptance

- **Fitness function (browser):** a committed browser spec under `tests/browser/` drives `/`,
  submits a query, asserts a known record appears, clicks it, and asserts its body renders. —
  `verify_by: browser`
- Scripted HTTP assertion: `GET /?q=<term>` returns 200 with the expected record present in
  the HTML for a seeded corpus. — `verify_by: command`
- **Fitness function (read-only):** an HTTP test asserts there is no route that mutates the
  store. — `verify_by: command`

### Out of scope

No in-browser editing (ADR 0006), no JSON API, no multi-project filter (slice 0005), no
auth. Renders nothing without a built read-model — that is expected.

### Plan

`web` crate (axum + templates) → search route → record route → browser spec + HTTP assertion.
Sub-slice if templates + routes + specs exceed the cap.
