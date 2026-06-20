---
type: Issue
title: Implement shorten + redirect endpoints
description: Build the two Phase 1 endpoints against the SQLite store.
status: open
labels: [phase-1, backend]
blocked_by: []
tracker:
timestamp: 2026-06-20T00:00:00Z
---

<!-- Everything below the closing --- is the issue body and stays byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Implement shorten + redirect endpoints

Build the `POST /shorten` and `GET /{code}` endpoints. Implements
[PRD 0001](/prd/0001-link-shortening.md), realizes [BDR 0001](/bdr/0001-shorten-and-redirect.md),
and uses the store from [ADR 0002](/adr/0002-sqlite-store.md).

### Scope

- Included: the two endpoints, URL-scheme validation, the SQLite store adapter.
- Explicitly KEPT out: analytics, accounts, rate limiting (Phase 2).

### Acceptance

Each maps to a BDR scenario so "done" is machine-checkable:

- BDR 0001 / Scenario 1 — minting a valid URL returns `201` + a code.
- BDR 0001 / Scenario 2 — a round trip redirects to the original.
- BDR 0001 / Scenario 3 — an unknown code returns `404`.
- BDR 0001 / Scenario 4 — a `javascript:` target returns `400` and stores nothing.

### Plan

1. `schema.sql` + `store.py` (the ADR 0002 adapter).
2. `POST /shorten` with scheme validation.
3. `GET /{code}` redirect / 404.
4. Translate the four BDR scenarios into tests; wire `tests/test_store_durability.py`.
