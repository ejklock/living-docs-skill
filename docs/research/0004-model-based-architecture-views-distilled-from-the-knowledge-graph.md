---
type: Research
title: Model-based architecture views distilled from the knowledge graph
description: "Direction registered, not decided: generate structural architecture views (module/container) from the ADR 0031 typed edge set and ADR 0032 declared lineage, the living-docs counterpart of Structurizr DSL / LikeC4 model-as-SSOT — eliminating cross-view drift for views that have a code oracle."
status: Draft
timestamp: 2026-08-14T08:06:39Z
---

# 0004. Model-based architecture views distilled from the knowledge graph

## Question

Can the structural architecture views (module layout, containers) be *generated* from a
model the repository already carries — the typed bi-temporal edge set of
[ADR 0031](/adr/0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md)
and the declared doc–code lineage of
[ADR 0032](/adr/0032-doc-code-lineage-is-declared-not-inferred.md) — instead of being
hand-drawn Mermaid that only the no-drift rule keeps honest?

## Method

Industry survey, August 2026: read the model-based "architecture as code" toolchains
(Structurizr DSL, LikeC4) against the hand-drawn diagrams-as-code approach (Mermaid,
PlantUML) this project uses, and against
[ADR 0036](/adr/0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md)'s
per-view instrument binding (`rules/architecture-diagrams.md`). Findings are true as of
2026-08-14.

## Findings

- Structurizr DSL and LikeC4 converge on one stance: the *model* is the single source of
  truth and every view is a projection of it, so cross-view drift is unrepresentable —
  a rename in the model updates every diagram that mentions the element [1][2].
- Per-diagram code (Mermaid) keeps each diagram individually versionable but shares no
  model between diagrams: consistency *between* views remains manual discipline, which is
  exactly the drift class the no-drift maintenance rule carries today.
- The project already stores a queryable structural model — ADR 0031's typed edge set —
  and ADR 0032 keeps its doc–code lineage declared rather than inferred, so a
  deterministic `living-docs arch distill` (edges → Mermaid `flowchart`) would violate no
  determinism boundary: same inputs, same diagram, no LLM.
- `rules/architecture-diagrams.md` already prefers "distill-from-code where an oracle
  exists"; this direction mechanizes that preference for the views whose oracle is the
  graph (component/module, container), leaving sequence/state views hand-authored — they
  have no oracle.

## Implications

If pursued, the natural shape is a CLI verb that regenerates the body of a
`kind: component` (and possibly `container`) view from the stored edge set, keeping the
view file's frontmatter and orientation prose hand-owned. This is a *direction
registered, not a decision*: no ADR locks it, no slice is scheduled, and the trade-off
(a second projection pipeline to maintain vs. drift-free structural views) has not been
steelmanned. Locking it requires an ADR that passes the decision-epistemics gate.

## Open Questions

- Does the ADR 0031 edge set carry enough module-granularity edges to render a useful
  component view, or would it need a code-derived import graph first?
- Should a distilled view be read-only (regenerated on `index`) or a scaffold the author
  refines — and how would `check` tell the two apart?

# References

[1] [Structurizr — Why "as code"?](https://docs.structurizr.com/as-code)
[2] [LikeC4 — architecture as code](https://likec4.dev/)
