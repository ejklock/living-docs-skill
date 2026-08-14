<!-- OKF reserved index.md (§6): a directory listing — NO frontmatter. The view files
     it links ARE concepts (`type: Architecture View`), scaffolded by
     `living-docs new view "<title>" --kind <kind>` and listed by `living-docs index`
     in C4/arc42 zoom order (ADR 0036). Everything above the first row is preamble the
     generator preserves; the rows below it are generator-owned — never hand-edit them. -->

# Architecture

Living architecture of the project, one view per concern, sequenced outside-in:
context, containers, components, then runtime behavior (flows, sequences, states),
data, and deployment. Each view names its drift instrument; update the relevant view
in the same change as any structural code change (see `rules/maintenance-invariant.md`).

* [Context](context.md) - context
* [Backends](backends.md) - container
* [Module layout](modules.md) - component
* [Request round trip](request-round-trip.md) - sequence
* [Data model](data-model.md) - data-model
