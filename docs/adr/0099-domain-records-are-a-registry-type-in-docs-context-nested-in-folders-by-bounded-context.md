---
type: ADR
title: Domain records are a registry type in docs/context, nested in folders by bounded context
description: write new domain scaffolds a living Context record with Terms and Rules under any folder depth of docs/context, the glossary stays as a record, and manage index generates the context index.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-23T02:23:42Z
---

# 0099. Domain records are a registry type in docs/context, nested in folders by bounded context

## Context

A project's domain knowledge (its terms and its business rules) has no CLI-owned home. [ADR 0027](0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md) kept the glossary a template rather than a registry row, so the `docs/context/` files consumers keep are hand-written: two consumer bundles hold 4 and 22 of them, every one with `type: Context`, a hand-kept `index.md`, and statuses that drift (`Accepted`, `current`, none). A design review on 2026-11-22 found that no open issue in either bundle links a rule in `docs/context/`, that one of them keeps invariants inside bullets with no addressable heading, and that the same term is defined in two files (`Lesson` in `glossary.md` and `domain-concepts.md`). Issue 0144 planned a trial before a type; the owner chose the type at once and asked that an author can group records in folders, as software engineering groups a domain by bounded context.

## Decision

We will add a `domain` row to the doc-type registry whose records are the files under `docs/context/`, with `type: Context` in their frontmatter, so the files consumers already keep become records without moving and every link to them keeps resolving.

- `living-docs write new domain "<Title>" [--dir <path>]` scaffolds `docs/context/<path>/<slug>.md`, the slug from the title. `<path>` is any depth of slug segments under `docs/context/` (one folder per bounded context, by the author's choice); an absolute path, `..`, an empty segment or a segment that is not already a slug is refused. Without `--dir` the record sits at the root of `docs/context/`.
- The scaffold carries frontmatter (`type: Context`, title, description, timestamp) and two optional sections, `## Terms` and `## Rules`. A rule is one `###` heading under `## Rules`; its heading is its name and its link anchor. The record has no number and no status vocabulary: it is a living record updated in place, like an architecture view, and git history is its trail.
- `living-docs manage index` generates `docs/context/index.md`, listing every record by folder in path order with its title and description; the hand-kept index is replaced by the generated one.
- The glossary stays. `glossary.md` and every other existing context file become records of this type in place; which file owns a term stays the author's choice.

This reverses the part of ADR 0027 that kept the glossary out of the registry; its size-target decision stands.

## Consequences

- An author or an agent creates a domain record, in the folder of its bounded context, through the CLI, and the index never drifts.
- A consumer's first `check` after upgrading reports its hand-kept `docs/context/index.md` as differing from its regeneration; `living-docs manage index` repairs it. A `status:` key on an existing context file raises the `status-vocabulary` advisory until its author removes it.
- The `Named` identity, which places only `<dir>/<slug>.md`, gains nested placement for this row; the generated index groups by folder instead of by a `kind` order.
- Deep nesting is allowed, so each bundle sets its own folder taxonomy; the CLI validates the segments but never chooses them.
- The CLI never writes a term or a rule, never decides which context owns one, and never reads a rule's meaning.
- Fitness functions: `write new domain` with and without `--dir` passes `check`; `manage index` over `docs/context/` is idempotent; a hostile `--dir` (absolute, `..`, a symlink out of the bundle) writes nothing.
