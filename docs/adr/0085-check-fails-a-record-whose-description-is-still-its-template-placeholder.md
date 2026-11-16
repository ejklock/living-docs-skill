---
type: ADR
title: check fails a record whose description is still its template placeholder
description: check reports a description equal to a doc-type template placeholder as an unfilled-slot violation; new keeps writing the placeholder; the repository fills its 18 placeholder descriptions in the same change
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-16T02:33:32Z
---

# 0085. check fails a record whose description is still its template placeholder

## Context

`write new` scaffolds every body section as a `{{SLOT: hint}}`, and `check` fails an unfilled slot. The `description` frontmatter key escapes that rule: `new` writes the template's placeholder sentence (`<One sentence — the change and its motivation.>` for an issue, `<One sentence — the decision and its scope.>` for an ADR), and `check` accepts it. The description is what `read` prints for each record, so an agent orienting itself reads the placeholder instead of the record's point. On 2026-11-15, 18 records in this repository's `docs/` still carried it (issue 0105).

## Decision

We will have `check` report a record whose `description` is still its template's placeholder as a violation, the same class as an unfilled body slot. The rule matches when the trimmed description equals the placeholder sentence of any registered doc type's template; a description that only quotes that text among other words passes. `write new` keeps writing the placeholder unless `--description` is given, so the authoring flow stays new, then fill, then `check`.

The change fills the description of every record in this repository that still carries the placeholder, so `docs/` stays green.

Rejected: `write new` leaving the key out until `write set` fills it. It writes no placeholder, but `check` does not require the key, so a record would pass with no description at all and `read` would print nothing for it.

Rejected: keeping it as it is. It costs nothing, but `read` keeps printing placeholders and the gate keeps two rules for one kind of unfilled slot.

## Consequences

**Easier / gained:**
- Every record `check` accepts has a real one-line description for `read` to print.
- One rule for unfilled template text, in the body and in the frontmatter.

**Harder / accepted trade-offs:**
- An adopter's bundle that carries placeholder descriptions starts failing `check` on upgrade; the message names the record and the fix (`write set <TYPE/NNNN> description --from -`).

## Verification

**Implementation impact:** `living-docs-core/src/check/` (the description rule and its message), the doc-type templates as the source of the placeholder text, and the 18 records in `docs/`.

**Verification criteria:**
- A record whose description is its template's placeholder fails `check`; a filled description and one that only quotes the placeholder text pass; each case has a test.
- `living-docs check` passes over `docs/`.
