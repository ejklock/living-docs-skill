---
type: Issue
title: Identity cannot express an author-named record in a directory, so glossary and the Context family stay hand-authored
description: Add a third Identity variant for records that live in a directory but are named by their author rather than numbered by the tool, so glossary and the Context family become creatable.
status: closed
timestamp: 2026-07-30T20:21:19Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Identity cannot express an author-named record in a directory

> **CLOSED 2026-09-22 — the structural gap was filled; the glossary row is
> not wanted.** The missing variant shipped as `Identity::Named { dir }` in
> [ADR 0036](../adr/0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md):
> a record at `<dir>/<slug>.md`, keyed by its concern, updated in place. It
> carries the Architecture View type, `new` scaffolds it without a number,
> `index` renders it, and `check` reaches it through `spec_for_dir` with no
> identity-specific branch — the fitness function this issue asked for. What
> stays undone is the `glossary` row, and it stays undone deliberately:
> [ADR 0027](../adr/0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md)
> already ruled glossary out as a doc type, and
> [ADR 0059](../adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md)
> cut the periphery rather than grew it. A future type that needs the shape
> inherits it from the registry and needs no new record here.

Follow-up from [ADR 0027](../adr/0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md).

`DocTypeSpec::identity` has two variants. `Numbered { dir }` places a record in a
directory under an `NNNN` prefix the tool allocates. `Singleton { file }` places it at one
fixed path. A third shape exists in the corpus and neither variant fits it: a record that
lives in a directory, has many peers, and is named by its author rather than numbered by
the tool.

Three shipped artifacts are waiting on it. The `glossary` template declares
`type: Context` and is one vocabulary group file among many — `context/glossary.md` and
`context/billing.md` are peers, not a first and a second. The `Context` family generally,
and the `Architecture View` family behind `architecture-index.md`, have the same shape.
All three are hand-copied today: there is no `living-docs new` verb that produces them,
which is the gap ADR 0026 closed for `constitution` and left open here.

The cost of the gap is not only ergonomic. A hand-copied record starts outside the
canonical-frontmatter invariant and outside index membership, so the two checks that keep
the numbered types honest do not apply to it.

### Scope

Included: a third `Identity` variant for author-named records in a directory, and
whatever `new`, `index`, `check` and the web front need in order to honor it. Wiring the
`glossary` template as the first row that uses it.

Explicitly kept: `Numbered` and `Singleton` keep their current meaning and their current
rows. This adds a variant; it does not reshape the two that exist.

Explicitly out: `architecture-index.md` and `context-index.md` stay unwired regardless.
Both declare themselves OKF reserved `index.md` files carrying no frontmatter — they
scaffold a directory and are not records, so no identity variant applies to them.

### Acceptance

- `DOC_TYPES` contains a `glossary` row, and `living-docs new glossary "..."` — or
  whatever naming the design settles on — produces a record that `living-docs check`
  passes without hand-editing.
- The record produced is subject to the canonical-frontmatter invariant and to whatever
  index rule the design chooses, and a test pins which of the two applies.
- Fitness function: the check layer learns the new variant from the registry, in the shape
  `is_bundle_singleton` already uses — adding a second author-named row requires no edit
  inside `check`.
- `living-docs check docs` and `living-docs check examples/linkly/docs` stay green.

### Plan

Design first: an ADR settling what the tool owns for an author-named record. The open
questions are the naming rule (does the tool derive the filename from the title, or accept
it verbatim?), index membership (is a `Context` group file listed in an index the tool
regenerates, given that `context-index.md` is authored prose rather than a generated
listing?), and whether `type:` collision matters now that two rows could share one
frontmatter value — `spec_for` and `spec_for_dir` key on token and directory, so nothing
resolves a spec from its frontmatter value today, but that is an unstated invariant rather
than a guaranteed one.

Then one slice per question the ADR closes, in the order registry variant → `new` →
`check`, so each is demoable on its own.
