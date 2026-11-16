---
type: ADR
title: Owner is a CLI-owned frontmatter field with a warn-then-error ratchet on ADR and BDR
description: owner enters the doctype registry as an optional CLI-owned frontmatter field, warn-then-error required on ADR and BDR via check --require-owner
owner: Evaldo Klock
status: Accepted
timestamp: 2026-08-27T19:10:25Z
---

# 0043. Owner is a CLI-owned frontmatter field with a warn-then-error ratchet on ADR and BDR

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

An artifact with no accountable person drifts, and agents propagate drifted decisions at machine speed ([issue 0036](../issues/0036-owner-field-in-record-frontmatter-required-for-adr-and-bdr-by-check.md)). Frontmatter is CLI-owned by construction (ADR 0021) and its shape comes from the single doctype registry (ADR 0026), so a new field must enter through the registry and get a mutation verb, mirroring how `status` and `describe` work. The existing corpus has no owner values, so an immediate hard requirement would break every conformant tree.

## Decision

We will add `owner` as an optional, CLI-owned frontmatter field on every doc type, required on ADR and BDR through a ratchet.

1. **Field.** `owner: <free-form string>` (a name or an email). The tool never validates it against any directory; identity resolution is a human concern.
2. **Verbs.** `new --owner <value>` sets it at creation; a new `living-docs owner <ref> <value>` verb mutates it, reusing the record-resolution and frontmatter-mutation helpers `status`/`describe` use. Hand-edits stay blocked by the existing hook.
3. **Ratchet.** `check` reports a missing `owner` on ADR and BDR as a warning by default. A bundle-level opt-in (`check --require-owner`) promotes it to an invariant violation. Once the corpus is backfilled, CI adopts the flag; the flag becoming default is a later, separate decision.
4. **Registry.** The requirement lives as a per-doctype row property in the doctype registry (ADR 0026), not as a hardcoded type list in `check`.

## Consequences

**Easier / gained:**
- Every decision record can name who answers for it; `check` makes the gap visible before it drifts.
- The ratchet lets the rule land green today and tighten without a breaking change.

**Harder / accepted trade-offs:**
- A free-form string can go stale (people leave). Accepted: validating identity is out of the tool's determinism boundary.
- One more frontmatter field for `fmt` canonicalization and export round-trip to carry.

**Follow-ups:**
- Backfill the ADR/BDR corpus, then flip CI to `--require-owner`.
- Issue 0037 consumes owner coverage as a Governed-attribute signal.

## Verification

<!-- OPTIONAL — include when this decision must be honored in code, so the doc closes the
     doc → implement → verify loop an agent (and any review step) can consume. Omit
     for a purely advisory record. Keep criteria checkable, not aspirational.
     Implementation impact: files / modules this decision touches, e.g. `src/store.py`.
     Fitness function: the test / lint / arch-unit assertion that fails if the second
     verification criterion is violated (see `rules/adr-conventions.md` rule 6). -->

**Implementation impact:** doctype registry (`living-docs-core`), `new` (`--owner` flag), new `owner` verb in `cli/src/commands/`, `check` warning + `--require-owner`, `fmt` canonical ordering, db-store export/round-trip of the field.

**Verification criteria:**
- `new adr "x" --owner alice` emits `owner: alice`; `owner <ref> bob` rewrites only that field and the record still passes `check`.
- `check` warns on an ADR/BDR without `owner`; `check --require-owner` fails on the same tree; non-ADR/BDR types never warn.
- Fitness function: export → re-import round-trip preserves `owner` byte-identically (ADR 0007 lossless round-trip suite extended).

# References

[1] [Making Your Data Ready for Agentic AI — Sadalage & Chandrasekaran](https://martinfowler.com/articles/making-data-ready-for-agentic-ai.html)
