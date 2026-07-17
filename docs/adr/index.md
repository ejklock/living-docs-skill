# ADRs

Architecture decisions for the Living Docs skill itself — dogfooding the conventions this
repo teaches. The decision *log* is this listing plus each record's `status` /
`superseded_by` frontmatter.

> **Active view (the corpus-at-scale convention).** Split the listing by `status` so a
> reader sees what is *in force* without scrolling through history. Superseded records are
> kept — never deleted — but parked below. See `skills/living-docs/rules/adr-conventions.md`.

## Active

* [0001 — A living-docs CLI that owns the deterministic layer of doc authoring](0001-living-docs-cli.md) - Accepted
* [0002 — Extract a hexagonal living-docs-core inside a Cargo workspace](0002-hexagonal-core-workspace.md) - Proposed
* [0003 — Storage backend is config-selected, mutually exclusive, and both modes authoritative](0003-storage-backend-model.md) - Proposed
* [0004 — db-mode runs on ParadeDB by default with SQLite opt-in, over SeaORM](0004-db-engine-and-data-layer.md) - Proposed
* [0005 — Normalized DB schema — projects root, typed records with an EAV tail, typed relations](0005-normalized-schema.md) - Proposed
* [0006 — The web view is a read-only axum server reusing living-docs-core](0006-web-read-only-axum.md) - Proposed
* [0007 — db-mode authoring data model and lossless export contract](0007-db-mode-authoring-data-model-and-lossless-export-contract.md) - Proposed
* [0008 — BDR carries a required Contract section (public API + agent tool schemas)](0008-bdr-contract-section.md) - Accepted
* [0009 — Document visibility is default-deny frontmatter data, validated and index-aware](0009-document-visibility-model.md) - Accepted
* [0010 — Public export is a deterministic allowlist build with a leak gate; publish is a human-gated procedure](0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md) - Accepted
