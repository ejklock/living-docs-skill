---
type: Issue
title: describe and status resolve record numbers ambiguously across doc-type directories
description: Bare record numbers are only unique per directory, so describe/status silently mutate the first registry match — add a type-qualified reference and fail on ambiguity. Cannot self-describe via CLI precisely because 0025 collides with ADR 0025.
status: closed
timestamp: 2026-08-05T20:18:36Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## describe and status resolve record numbers ambiguously across doc-type directories

`living-docs describe <NNNN>` (and by shared record-resolution, `status`) resolves a bare number against the doc-type registry in order and stops at the first match. Numbers are only unique per directory, so a number that exists in more than one directory silently mutates the wrong record. Observed in practice on 2026-08-05: `living-docs describe 0003 "<research description>"` overwrote the description of ADR 0003 (`docs/adr/0003-storage-backend-model.md`) when the intended target was research 0003. The write succeeded silently; only `git diff` revealed the wrong target.

### Scope

- Accept a type-qualified record reference on every number-resolving verb (`describe`, `status`, `supersede`, future `covers`/`commits`): a type token prefix such as `research/0003` or `adr 0003` — exact grammar to be decided with the fix, reusing the registry's `token` field.
- When a bare number matches records in more than one directory, fail with the candidate list instead of picking the first match. A bare number that matches exactly one directory keeps working unchanged.
- KEPT: single-directory resolution behavior, the shared record-resolution helper (one fix covers all verbs), and the CLI-owned frontmatter contract.

### Acceptance

- With a number present in two directories, the bare-number form exits non-zero and lists both candidates; no file is modified.
- The type-qualified form mutates exactly the record in the named directory, demonstrated by a test fixture with colliding numbers across `adr/` and `research/`.
- A bare number unique to one directory resolves exactly as today (regression fixture).

### Plan

Single slice: extend the shared record-resolution helper with an optional type token + ambiguity detection, update `describe`/`status`/`supersede` argument parsing, add the colliding-number fixtures.
