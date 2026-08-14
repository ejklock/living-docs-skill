---
type: ADR
title: Architecture views are a registry doc type on a Named identity with a kind-sequenced generated index
description: "Promote architecture views to CLI-owned records: a new Named identity (dir, slug-named files) carries a view doc type in docs/architecture/, scaffolded by new --kind, canonically checked, and indexed in C4/arc42 zoom order by a kind frontmatter key."
status: Proposed
timestamp: 2026-08-14T07:53:45Z
---

# 0036. Architecture views are a registry doc type on a Named identity with a kind-sequenced generated index

## Context

`rules/architecture-diagrams.md` already prescribes the right shape — a `docs/architecture/`
directory, one view per file, an index as entry point — but architecture is the only
artifact the CLI does not own: no `DOC_TYPES` row, so no scaffold, no generated index, no
canonical-frontmatter check, no authoring hook. Without the cheap mechanical path every
other record type gets, real bundles regress to a single `architecture.md`, which is the
anti-pattern the literature has rejected since the multi-view consensus (Kruchten 4+1,
Views-and-Beyond, ISO/IEC/IEEE 42010: no single view serves every stakeholder concern).
Industry practice as of 2026 organizes the views by C4 zoom levels sequenced in arc42
chapter order — the vocabulary `rules/architecture-diagrams.md` already adopts as a
completeness checklist.

Epistemic type: **Judgment** — an artifact-shape trade-off with no experiment available;
no number is attached. External critic: **pending** (real authoring usage qualifies).

## Decision

We will add a third identity variant, `Named { dir }` — records live at
`<dir>/<slug>.md`, slug from the title, no number — and register a `view` doc type on it:
`type: Architecture View`, directory `architecture`, empty status vocabulary (views are
living documents: updated in place, never superseded; git history is the trail).
`living-docs new view "<title>" --kind <kind>` scaffolds a view carrying a validated
`kind` frontmatter key from the closed vocabulary `context | container | component |
flow | sequence | state | data-model | deployment`; `living-docs index` regenerates
`architecture/index.md` with rows sorted by that C4/arc42 zoom order (kind rank, then
filename; unknown or absent kind sorts last). The canonical-frontmatter check and the
docs-handwrite hook extend to the `architecture/` directory.

## Consequences

**Easier / gained:**
- The multi-view layout stops being prose advice and becomes the mechanical default — scaffold, index, and check all push toward one view per concern.
- The generated index reads in zoom order (context → container → component → runtime → data → deployment) without hand-maintenance.
- `record.rs` already classifies non-numbered types as concept-identity, so db-store ingestion and search need no schema change.

**Harder / accepted trade-offs (the declared sacrifice):**
- A third `Identity` variant widens every identity match in core. Steelman of the
  rejected alternative — numbered views (`NNNN-<slug>.md`) would reuse the existing
  machinery wholesale; rejected because numbering encodes an append-only, supersedable
  history that views do not have: a view is keyed by its concern and updated in place.
- Keeping views out of the registry (status quo, prose-only convention) was rejected on
  observed behavior: unowned artifacts regress to single files.
- Views escape the status/supersede lifecycle entirely — acceptable because their
  lifecycle is the code's, but it means no `status` verb signal distinguishes a live view
  from an abandoned one; the no-drift maintenance rule carries that weight.

**Follow-ups:**
- `templates/architecture-view.md` (registry-embedded), `templates/architecture-index.md`
  aligned to the generated format, `rules/architecture-diagrams.md` updated to the
  CLI-first flow.
- Model-based view distillation (generate the module view from the ADR 0031 knowledge
  graph / ADR 0032 declared lineage) is recorded as a research direction, not decided
  here.

## Verification

**Implementation impact:** `living-docs-core/src/doc_type.rs` (variant + row),
`commands/{new,brief,index}.rs`, `check/{mod,canonical}.rs`, `paths.rs`,
`skills/living-docs/hooks/block-docs-handwrite.sh`, templates and rules.

**Verification criteria:**
- `new view "Container View" --kind container` writes `docs/architecture/container-view.md`
  that passes `check`; an unknown `--kind` is refused with the vocabulary listed.
- `index` renders `architecture/index.md` sorted by kind rank then filename, idempotently.
- Fitness function: registry fitness tests extend to the `view` row (template/frontmatter
  round-trip); index tests pin the kind order; hook parity tests cover `architecture/`.

# References

[1] [C4 model (BROWN) — hierarchical zoom levels](https://c4model.com/)
[2] [arc42 (STARKE; HRUSCHKA) — section order mapped to C4 views](https://arc42.org/)
[3] [ISO/IEC/IEEE 42010 — architecture description, concern-to-view traceability](https://www.iso.org/standard/74393.html)
