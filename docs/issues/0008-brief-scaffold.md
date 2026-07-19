---
type: Issue
title: living-docs brief — deterministic pre-filled scaffold that leaves only the judgment slots for the authoring model
description: A `brief` subcommand that extends `new` by pre-filling everything derivable from the repo (frontmatter, next number, timestamp, cross-links, git-derived touched files) and emitting the template with explicitly marked empty judgment slots — so the authoring model fills gaps instead of generating whole files.
status: open
labels: [slice, cli, token-economy]
blocked_by: []
tracker:
timestamp: 2026-07-19T00:00:00Z
---

## living-docs brief — pre-filled scaffold, judgment slots left empty

Motivated by ai-configs research `0053` (living-docs token economy): the authoring
model's cost is output tokens spent on prose; everything mechanically derivable should
come from the tool at zero token cost. `brief` is the next step on the ADR 0001 line —
it widens what the tool pre-fills without crossing the determinism boundary (the tool
derives facts; it never writes rationale, never chooses an epistemic type).

### Objective link

Constitution (deterministic layer owns the mechanical) → [ADR 0001](/adr/0001-living-docs-cli.md)
(CLI owns template-fillable steps) → this slice.

### Context manifest

- Read: `cli/src/main.rs` (`New`/`Next` commands), `living-docs-core` record templates,
  the fs `DocStore`.
- Seams touched: a `Brief` variant next to `New` in the CLI; a scaffold builder in
  `living-docs-core` that composes the existing template + derivable fields; optional
  git read (touched files for an issue/ADR context section) behind a flag so the core
  stays I/O-free.
- Pattern: strictly a superset of `new` — same template, more fields pre-filled, empty
  slots explicitly marked (e.g. `<!-- judgment: rationale -->`) so an agent can locate
  them without re-reading the whole file.

### Scope

`living-docs brief <doc-type> "<title>"` writes the same record `new` would, with:
number/slug/frontmatter/timestamp filled (as today), cross-link stubs to the docs the
type conventionally links (ADR → issue/research placeholders), an optional
`--from-diff <range>` that lists git-touched files into the context section, and every
judgment field emitted as a marked empty slot. Output passes `check`.

### Vertical Demo

- **Given** a docs bundle, **When** I run `living-docs brief adr "Choose X over Y"`,
  **Then** a conformant ADR file exists with frontmatter and links pre-filled and the
  Context/Decision/Consequences bodies as marked empty slots, and `living-docs check`
  passes.
- **Given** `--from-diff HEAD~3..HEAD`, **When** I run `brief issue "..."`, **Then**
  the context section lists the files touched in that range (unhappy path: an invalid
  range fails with a clear error, no file written).

### Acceptance

- `brief` output passes `check` for every doc type it supports. — verify_by: test
- `brief` writes no prose into judgment slots — the slots are byte-identical markers,
  asserted by test. — verify_by: test
- `--from-diff` content is exactly derivable from `git diff --name-only` on the range
  (deterministic; same input → same output). — verify_by: test
- Complexity + no-comments + tests-with-the-change standing rules hold. — verify_by: command

### Out of scope

No LLM calls, no prose generation, no epistemic-type inference (determinism boundary,
ADR 0001). No new ADR needed unless implementation surfaces a real fork; the scaffold
is additive CLI surface under existing decisions. Size-target warnings are slice
[0009](/issues/0009-doc-size-targets.md).
