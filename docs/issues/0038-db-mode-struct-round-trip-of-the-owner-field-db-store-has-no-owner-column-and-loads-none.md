---
type: Issue
title: "db-mode struct round-trip of the owner field: db-store has no owner column and loads None"
description: add a typed owner column to db-store so db-mode loads and exports the owner field instead of None; struct-level gap left by the owner-field slice
status: closed
timestamp: 2026-08-27T21:04:13Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## db-mode struct round-trip of the owner field: db-store has no owner column and loads None

Follow-up to [ADR 0043](../adr/0043-owner-is-a-cli-owned-frontmatter-field-with-a-warn-then-error-ratchet-on-adr-and-bdr.md). The owner field entered TYPED_FRONTMATTER_KEYS (excluded from the EAV frontmatter tail), but db-store has no typed owner column, so `load_record` fills `ExtractedRecord::owner` with `None`.

The gap is struct-level only: `check` and the `owner` verb read raw record text via `store.read()`, so no write path drops an existing owner value. What breaks is any consumer of the typed struct in db-mode, and ADR 0043's export round-trip fitness for db-authoritative deployments.

### Scope

- Add an `owner` column to the db-store records schema via a new migration; populate it on sync and authoring writes; read it in `load_record`.
- Extend the export round-trip suite so a db-mode record with owner materializes it byte-identically.
- KEPT: TYPED_FRONTMATTER_KEYS membership; fs-mode behavior unchanged.

### Acceptance

- A record synced with `owner: alice` loads with `owner == Some("alice")` in db-mode and exports byte-identically.
- Existing db-store tests stay green; the migration applies cleanly on both engines.

### Plan

1. Migration + entity column, sync/authoring write paths.
2. `load_record` read + round-trip test extension.
