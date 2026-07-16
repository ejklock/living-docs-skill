# ADRs

Architecture decisions for the Living Docs skill itself — dogfooding the conventions this
repo teaches. The decision *log* is this listing plus each record's `status` /
`superseded_by` frontmatter.

> **Active view (the corpus-at-scale convention).** Split the listing by `status` so a
> reader sees what is *in force* without scrolling through history. Superseded records are
> kept — never deleted — but parked below. See `skills/living-docs/rules/adr-conventions.md`.

## Active

* [0001 — A living-docs CLI that owns the deterministic layer of doc authoring](0001-living-docs-cli.md) - Accepted
