---
type: Issue
title: "status verb cannot set the issue lifecycle: number resolution prefers the ADR on cross-type collision and the status vocabulary is ADR-only"
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-08-06T20:33:19Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## status verb cannot set the issue lifecycle: number resolution prefers the ADR on cross-type collision and the status vocabulary is ADR-only

Closing issue 0028 exposed a resolution gap shared by `status` and `describe`. The record resolver maps a bare number to one record across types: `status 0028 closed` silently resolved to ADR 0028, not issue 0028, and failed only because `closed` is outside the ADR vocabulary. Worse, `describe 0029 "..."` silently overwrote ADR 0029's description when the target was issue 0029 — a wrong-record WRITE with no error. The per-type status vocabulary itself already exists ([ADR 0029](/adr/0029-the-status-vocabulary-is-per-doc-type-sourced-from-one-doctypespec-field-not-one-global-list-validated-against-every-template-s-own-dialect.md)); the missing piece is disambiguation. The hand-write hook correctly blocks a manual frontmatter edit, so an issue whose number collides with an ADR cannot leave `open` today.

### Scope

- Record resolution takes an explicit type qualifier (for example `status issue 0028 closed`) and fails loudly on an unqualified cross-type collision instead of silently picking one type.
- The shared resolution helper serves `status`, `describe`, and `supersede`, so all three gain the qualifier and the fail-loud behavior.
- KEPT: the per-type vocabularies (ADR 0029), the `Superseded`-via-`supersede` rule, and single-match resolution for non-colliding numbers.

### Acceptance

- `living-docs status issue 0028 closed` sets `status: closed` on issue 0028 while ADR 0028 stays untouched.
- An unqualified `status 0028 ...` or `describe 0029 ...` on a collision exits non-zero and names every candidate record; no file is written.
- `living-docs check` stays green; fixture tests cover the collision for `status` and `describe`.

### Plan

1. Extend the record resolver with an optional type qualifier and fail-loud collision handling.
2. Wire `status`, `describe`, and `supersede` through the shared helper; add fixture tests for both verbs on a colliding number.
