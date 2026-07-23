---
type: ADR
title: The web view is a read-only axum server reusing living-docs-core
description: The query web front is a Rust/axum server that reuses living-docs-core and reads the db-store read-model; it is read-only (no authoring in the browser), server-rendered, and never a second source of truth.
status: Superseded
superseded_by: 0016
tags: [architecture, axum, front, read-only, search, web]
timestamp: 2026-07-16T00:00:00Z
---

# 0006. The web view is a read-only axum server reusing living-docs-core

## Context

The North Star (Constitution) is findability: a developer finds the relevant ADR/PRD/concept
in seconds. Search over the read-model (ADR 0003/0004) delivers the query; a **web view**
makes it browsable — the slice-2 observable surface. This ADR fixes the web's shape.

Two forks matter. First, **read-only vs editing**: letting the browser author documents
would make the web a write path, reintroducing a second source of truth and the
sync/conflict problem ADR 0003 killed by keeping backends exclusive and authoring in the
CLI. Second, **server-rendered vs SPA**: a separate JS SPA adds a second stack, a second
build, and an API contract to version — cost the findability slice does not need.

`living-docs-core` already holds the domain and the `SearchIndex` port; the web should reuse
it, not reimplement a second notion of a record or a query.

## Decision

We will build the web view as a **read-only, server-rendered Rust/axum server that reuses
`living-docs-core`** and reads the `db-store` read-model through the `SearchIndex` /
`DocStore` ports:

- **Read-only.** It renders search results and record views. It never creates, edits, or
  deletes a document — authoring stays in the CLI. The web is never a source of truth.
- **Server-rendered** (HTML templates), same workspace, same language — no separate SPA
  stack, no API contract to maintain for v1.
- **Reuses the core** — one definition of a record and a query; the web cannot drift from
  the CLI.

An HTTP/JSON surface or a richer client is a later, evidence-gated decision, not v1.

## Consequences

**Easier / gained:**
- One language, one build, one binary path; the web cannot drift from the CLI's domain.
- No write path means no new source-of-truth or sync surface — ADR 0003's exclusivity holds.
- The findability slice ships with a real, openable surface behind a browser fitness function.

**Harder / accepted trade-offs:**
- No in-browser editing — an accepted limitation for v1; authoring is CLI-only.
- Server-rendered HTML is less interactive than an SPA — accepted; findability needs
  search + view, not a rich client.
- The web depends on a built read-model (file-mode) or the authoritative DB (db-mode) being
  present; it renders nothing without a store.

**Follow-ups:**
- Deferred, evidence-gated: an HTTP/JSON API and/or a richer client if a real need appears.

## Verification

**Implementation impact:** a `web` crate (axum + templates) reading the read-model via
`living-docs-core` ports; a route for search and a route for a record view.

**Verification criteria:**
- **Fitness function (browser):** a committed browser spec drives `/`, types a query, and
  asserts a known record appears in the results and its body renders on click.
- **Fitness function (read-only):** the server exposes no route that mutates the store — an
  HTTP test asserts there is no write endpoint.
- A scripted HTTP assertion on the search route returns the expected record for a seeded
  corpus.

# References

[1] ADR 0002 (core reuse), ADR 0003/0004 (the read-model the web reads).
