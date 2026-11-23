---
type: ADR
title: Every rule keyed by doc type becomes a registry field, and glossary is not a doc type
description: A row is a doc type the tool creates, numbers or places; a field is any rule it applies once the type is known — so the body-size rule moves into DocTypeSpec and glossary stays out.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-07-30T20:19:45Z
---

# 0027. Every rule keyed by doc type becomes a registry field, and glossary is not a doc type

## Context

[ADR 0026](0026-a-single-doctype-registry-replaces-nine-hand-synced-enumerations-and-research-and-constitution-enter-as-rows.md)
replaced nine hand-synced doc-type enumerations with one compile-time `DocTypeSpec`
registry, and deliberately left two questions open. Both are the same question — what
belongs in the registry — and they answer in opposite directions, which is why they are
settled together.

The first is `check::size::has_size_target`, deciding whether a record's body is measured
against the 100/120-line target:

```rust
fn has_size_target(doc_type: &str) -> bool {
    matches!(doc_type, "ADR" | "BDR" | "PRD" | "Issue")
}
```

This is a tenth enumeration, and the worst-behaved of the ten, because it fails *open*.
Every list ADR 0026 absorbed announced a missing type loudly — a panic, an unresolvable
directory, a missing index row. This one announces nothing: a new row is silently exempt,
and no reader is told a decision was made on their behalf. The test meant to guard it,
`every_decision_and_execution_record_type_carries_the_target`, iterates the literal
`["ADR", "BDR", "PRD", "Issue"]` — mirroring the implementation instead of constraining
it, so it cannot fail when a type is added.

ADR 0026 deferred it because a size target "encodes an epistemic category, not identity".
That reason does not survive contact with the registry as built: `DocTypeSpec` already
carries `frontmatter`, `template` and the index metadata, none of which are identity
either, and the row's own docblock says it holds everything a doc-type consumer needs.
`Identity` carries identity — the rest of the row is precisely this kind of rule.

The second is the `glossary` template, which ships under `skills/living-docs/templates/`
with no way to create the record it describes — the half-wired state `constitution` was in
before ADR 0026. The obvious move is to make it a row. Reading it first says otherwise:

```yaml
type: Context
title: "Glossary"
```

It declares `type: Context`, not `type: Glossary`, and `Context` is a category with many
members: the shipped `context-index.md` states that each group file "owns one coherent
slice of the vocabulary". A glossary is one such slice, not a type of its own, and it has
no number and no fixed path — `context/glossary.md` and `context/billing.md` are peers.

`Identity` cannot express that shape. `Numbered { dir }` gives a directory and an `NNNN`
prefix; `Singleton { file }` gives one fixed path. A Context group file is plural like the
first and unnumbered like the second: an author-named record inside a directory. Wiring
`glossary` does not need a row, it needs a third identity variant — a larger decision,
with its own consequences for `new`, `index` and `check`, that this ADR does not make.

The two remaining unwired templates are not candidates at all: `architecture-index.md` and
`context-index.md` each declare themselves OKF reserved `index.md` files carrying no
frontmatter — directory scaffolds, not records.

## Decision

We will move every rule keyed by doc type into `DocTypeSpec`, beginning with the body-size
rule, and we will not make `glossary` a registry row.

> Amended by [ADR 0099](0099-domain-records-are-a-registry-type-in-docs-context-nested-in-folders-by-bounded-context.md): the context files, the glossary among them, became records of a `domain` row in `docs/context/`. The size-target decision below stands.

`has_size_target` becomes a `body_size` field on the row, typed as an enum rather than a
`bool` so that a call site reads as a rule and a future third rule stays representable.
`check::size` reads that rule through the registry instead of matching on strings, and its
guard test iterates `DOC_TYPES` rather than a literal list — the fitness-function shape
ADR 0026 established. Because the field is required, a new row cannot compile without
stating its body-size rule, which turns a silent default into a decision.

The registry's boundary is thereby stated positively, and applies to the next candidate
without re-deriving it: **a row is a doc type the tool creates, numbers or places; a field
is any rule the tool applies once it knows the type.**

`Identity` decides the first; everything else on the row is the second. `glossary` fails
the row test — the tool cannot place it, because its placement is the author's choice —
so it stays unwired until `Identity` grows a variant for an author-named record in a
directory.

## Consequences

**Easier / gained:**
- Adding a doc type is still one row, and it will not compile until its body-size rule is
  stated — the one list that could fail open no longer can.
- "Is a Research doc exempt?" is answered by reading the Research row rather than by
  grepping the check module, and the next unwired template is decided by a stated
  admission rule rather than by argument.

**Harder / accepted trade-offs:**
- The `Context` and `Architecture View` families stay hand-authored, so the gap ADR 0026
  closed for `constitution` stays open for `glossary`. Accepted rather than force a shape
  the registry cannot hold: a wrong row costs more than a missing one, because it becomes
  a premise every consumer reads as true.
- A registry row now carries a check-layer concern. Deliberate coupling — rules keyed by
  type are cheaper to keep honest in one table than in N modules, and `check::size` still
  owns the thresholds themselves.

**Follow-ups:**
- [Issue 0018](../issues/0018-identity-cannot-express-an-author-named-record-in-a-directory-so-glossary-and-the-context-family-stay-hand-authored.md)
  for the third `Identity` variant, which is what `glossary` and the wider `Context` and
  `Architecture View` families are waiting on.
- The 100/120-line thresholds stay global constants in `check::size`. Per-type thresholds
  become representable in the new field, but no type wants them and inventing the need
  would reintroduce the configurability this registry exists to avoid.

## Verification

**Implementation impact:** `living-docs-core/src/{doc_type.rs, check/size.rs}`.

**Verification criteria:**
- `has_size_target` no longer exists, and no literal doc-type list or `matches!` over type
  names remains anywhere in `living-docs-core/src/check/size.rs`.
- Fitness function: a test iterates `DOC_TYPES` and asserts, for every row, that a body one
  line over the warn threshold is flagged exactly when that row's `body_size` rule says it
  should be — replacing `every_decision_and_execution_record_type_carries_the_target`, not
  sitting beside it. Adding a row without deciding fails to compile; deciding wrongly
  fails this test.
- `living-docs check docs` and `living-docs check examples/linkly/docs` report the same
  violation counts as before — a refactor of where the rule lives, not of what it decides.
- `DOC_TYPES` contains no `glossary` row.
