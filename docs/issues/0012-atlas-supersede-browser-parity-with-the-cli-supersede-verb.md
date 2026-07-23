---
type: Issue
title: Atlas supersede — browser parity with the CLI supersede verb
description: Adds an Atlas supersede action reusing living-docs-core's existing supersede verb, so superseding via the browser leaves both records linked and conformant identically to the CLI path.
status: open
labels: [web, atlas, authoring, database]
blocked_by: [10, 11]
timestamp: 2026-07-21T00:00:00Z
---

## Atlas supersede — browser parity with the CLI supersede verb

Implements the supersede-parity verification criterion of [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md):
"superseding via the Atlas UI leaves both records linked and conformant, identical to the
CLI path." Unlike create/edit, `living-docs-core` already has a supersede verb (`--backend
db supersede`, issue 0006) — this slice exposes it through Atlas, it does not invent new
core logic.

### Objective link

Constitution → [PRD 0001](/prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md)
→ [ADR 0016](/adr/0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
→ issues 0010/0011 → this slice.

### Context manifest

- Read: `living-docs-core`'s existing `supersede` service (issue 0006/ADR 0007), issue
  0008's metadata panel (already renders the supersede chain read-only).
- Seams touched: a supersede action on the record page (pick the superseding record,
  confirm), wired to the same core `supersede` verb the CLI uses — reused, not
  reimplemented — inside the transactional write+check wrapper from issue 0010 so a
  supersede that would dangle a link is refused, exactly as the CLI's `check` refuses it.
- Pattern: one core verb, two callers (CLI, Atlas) — the ADR 0002 port-seam payoff.

### Scope

- `web`: a supersede action on the record page (author picks or types the superseding
  record's number/path, confirms); on success the page re-renders showing the updated
  status badge and supersede chain (both already rendered by issue 0008's metadata panel).
- No new `living-docs-core` logic — this slice calls the existing supersede verb through
  the same transactional wrapper create/edit use, and surfaces its errors (e.g. "no record
  found for NNNN") in the browser instead of stderr.
- Browser spec: supersede via Atlas updates both records' status/links, matches
  `living-docs --backend db check`'s verdict, and a supersede naming a nonexistent record
  is rejected with the same error the CLI produces.

### Vertical Demo

- **Given** two records open in Atlas, **When** I supersede the older with the newer via
  the UI, **Then** both pages immediately reflect the new status/supersede chain and
  `check` reports no invariant violation for the pair.
- **Given** a supersede naming a record number that does not exist, **When** I submit it,
  **Then** Atlas shows the same "no record found" message the CLI returns, and neither
  record changes.

### Acceptance

- **Supersede parity (fitness function):** superseding via Atlas leaves both records
  linked and conformant, identical to the CLI path on the same corpus. — `verify_by: test`
- **Doc-gate on write:** a supersede that would dangle a link is refused inside the
  transaction; nothing commits. — `verify_by: browser`
- `living-docs check` and `cargo test --workspace` stay green. — `verify_by: command`

### Out of scope

Delete (issue 0013). Un-superseding / reverting a supersede — not a CLI capability today
either, so out of scope for parity.

### Plan

Single vertical slice: (a) supersede action + confirmation UI on the record page; (b) wire
to the existing core supersede verb via the transactional wrapper; (c) browser spec for
the happy path and the dangling-link rejection.
