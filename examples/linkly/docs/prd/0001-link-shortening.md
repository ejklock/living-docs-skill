---
type: PRD
title: Link shortening & redirect
description: Mint a short code for a URL and redirect that code back to the original.
status: Accepted
superseded_by:
tags: [phase-1]
timestamp: 2026-06-20T00:00:00Z
---

# 0001. Link shortening & redirect

## Problem / Motivation

People share long, fragile URLs in places with tight length limits or where a link must
survive being typed by hand. They need a short stand-in that reliably resolves to the
original and never breaks.

## Goals

- A user can turn any valid web URL into a short code in one request.
- Visiting the short code sends the user to the original URL.

## Non-goals

- Analytics, accounts, and vanity domains (out of scope per the
  [constitution](/constitution.md)).

## Requirements

1. The system accepts an `http`/`https` URL and returns a short code.
2. Visiting `/{code}` redirects to the stored `target_url`.
3. An unknown code returns "not found", never a redirect.
4. A non-`http(s)` target is rejected at mint time.

## Acceptance criteria

- A round trip (mint a code, then visit it) lands on the original URL.
- A `javascript:` target is refused with a client error.

## Success metrics

- ≥99% of mint requests for valid URLs succeed within 100ms (single region, Phase 1).

## Behavior (BDRs)

- [BDR 0001 — Shorten & redirect](/bdr/0001-shorten-and-redirect.md)

## Open questions

- Where are links stored, and does that survive a restart? → resolved by
  [ADR 0002](/adr/0002-sqlite-store.md).

## Decision log

- Storage: [ADR 0002 — SQLite store](/adr/0002-sqlite-store.md)
  (supersedes [ADR 0001](/adr/0001-in-memory-store.md)).

## Related

- Constitution: [/constitution.md](/constitution.md)
- Issues: [/issues/0001-implement-shorten-endpoint.md](/issues/0001-implement-shorten-endpoint.md)
