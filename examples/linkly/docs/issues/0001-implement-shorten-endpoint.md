---
type: Issue
title: Implement shorten + redirect endpoints
description: Build the two Phase 1 endpoints against the SQLite store.
status: open
labels: [phase-1, backend]
blocked_by: []
timestamp: 2026-06-20T00:00:00Z
---

<!-- Everything below the closing --- is the issue body and stays byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Implement shorten + redirect endpoints

Build the `POST /shorten` and `GET /{code}` endpoints. Implements
[PRD 0001](../prd/0001-link-shortening.md) and uses the store from
[ADR 0002](../adr/0002-sqlite-store.md).

### Scope

- Included: the two endpoints, URL-scheme validation, the SQLite store adapter.
- Explicitly KEPT out: analytics, accounts, rate limiting (Phase 2).

### Acceptance

Each is one test in the regression suite, so "done" is machine-checkable:

- Minting a valid URL returns `201` + a code.
- A round trip redirects to the original.
- An unknown code returns `404`.
- A `javascript:` target returns `400` and stores nothing.

### Plan

1. `schema.sql` + `store.py` (the ADR 0002 adapter).
2. `POST /shorten` with scheme validation.
3. `GET /{code}` redirect / 404.
4. Write the four acceptance tests; wire `tests/test_store_durability.py`.
