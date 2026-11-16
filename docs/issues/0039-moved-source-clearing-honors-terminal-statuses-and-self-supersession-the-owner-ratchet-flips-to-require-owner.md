---
type: Issue
title: moved-source clearing honors terminal statuses and self-supersession; the owner ratchet flips to require-owner
description: fix the two moved-source false-positive classes (done-status dependents and dependent-is-successor) and promote check to --require-owner in the pre-commit hook and CI make check
owner: Evaldo Klock
status: closed
timestamp: 2026-08-27T22:00:48Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## moved-source clearing honors terminal statuses and self-supersession; the owner ratchet flips to require-owner

The corpus sweep after [ADR 0044](../adr/0044-moved-source-review-queue-check-emits-a-warn-level-finding-when-a-linked-record-is-superseded-or-demoted.md) exposed two false-positive classes in the MOVED-SOURCE clearing rule. First, the closed-dependent clause matches only `superseded|closed`, so issues with status `done` — a terminal status in this bundle's issue vocabulary — still fire. Second, a record that links its own predecessor fires against itself: the finding names the dependent as the successor, and no link update can clear it. With those fixed and owners backfilled (repo and example corpus), the [ADR 0043](../adr/0043-owner-is-a-cli-owned-frontmatter-field-with-a-warn-then-error-ratchet-on-adr-and-bdr.md) ratchet flips to error.

### Scope

- Clearing rule: a dependent whose status is terminal for its doc type (registry-sourced, covering `done`) never fires; a finding whose successor IS the dependent never fires.
- Ratchet flip: `.githooks/pre-commit` and the CI `make check` doc-gate invocations gain `--require-owner`.
- KEPT: the finding's advisory severity, message shape, and every true-positive path from ADR 0044.

### Acceptance

- `living-docs check` over this repo's docs and over `examples/linkly/docs` emits zero MOVED-SOURCE advisories, with no body edit beyond the already-committed sweep.
- Unit tests cover both new clearing branches; existing moved-source tests stay green.
- `make check` and the pre-commit hook fail on an ADR/BDR record without an owner.

### Plan

1. Registry-sourced terminal-status clearing + dependent-is-successor exemption in the moved-source pass, with tests.
2. `--require-owner` in `.githooks/pre-commit` and the Makefile doc-gate lines.
