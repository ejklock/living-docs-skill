# ADRs

Architecture decisions for the Living Docs skill itself — dogfooding the conventions this
repo teaches. The decision *log* is this listing plus each record's `status` /
`superseded_by` frontmatter.

> **Active view (the corpus-at-scale convention).** Split the listing by `status` so a
> reader sees what is *in force* without scrolling through history. Superseded records are
> kept — never deleted — but parked below. See `skills/living-docs/rules/adr-conventions.md`.

## Active

* [0001 — A living-docs CLI that owns the deterministic layer of doc authoring](0001-living-docs-cli.md) - Accepted
* [0002 — Extract a hexagonal living-docs-core inside a Cargo workspace](0002-hexagonal-core-workspace.md) - Accepted
* [0003 — Storage backend is config-selected, mutually exclusive, and both modes authoritative](0003-storage-backend-model.md) - Accepted
* [0004 — db-mode runs on ParadeDB by default with SQLite opt-in, over SeaORM](0004-db-engine-and-data-layer.md) - Accepted
* [0005 — Normalized DB schema — projects root, typed records with an EAV tail, typed relations](0005-normalized-schema.md) - Accepted
* [0007 — db-mode authoring data model and lossless export contract](0007-db-mode-authoring-data-model-and-lossless-export-contract.md) - Accepted
* [0008 — BDR carries a required Contract section (public API + agent tool schemas)](0008-bdr-contract-section.md) - Accepted
* [0009 — Document visibility is default-deny frontmatter data, validated and index-aware](0009-document-visibility-model.md) - Accepted
* [0010 — Public export is a deterministic allowlist build with a leak gate; publish is a human-gated procedure](0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md) - Accepted
* [0011 — Secret and PII detection stays deterministic — a curated ruleset plus Shannon entropy, never ML](0011-leak-detection-stays-deterministic-curated-ruleset-plus-shannon-entropy-never-ml.md) - Accepted
* [0012 — Worldwide PII detection is checksum-tiered, two-stage, and deterministic](0012-worldwide-pii-detection-is-checksum-tiered-two-stage-and-deterministic.md) - Accepted
* [0013 — Mermaid validation runs in-process via merman-core, not a Docker mermaid-cli shell-out](0013-mermaid-validation-runs-in-process-via-merman-core-not-a-docker-mermaid-cli-shell-out.md) - Accepted
* [0015 — Web UX follows the three-pane doc-site archetype with a search-first Cmd+K palette](0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md) - Accepted
* [0016 — Atlas makes the web a db-mode authoring front, superseding web read-only](0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md) - Accepted
* [0017 — SKILL.md stubs are pure routers; the spine and all detail move to CLI topics](0017-skill-md-stubs-are-pure-routers-the-spine-and-all-detail-move-to-cli-topics.md) - Accepted
* [0018 — Atlas delete is a soft-delete, scoped to non-decision doc types, refused on inbound relations](0018-atlas-delete-is-a-soft-delete-scoped-to-non-decision-doc-types-refused-on-inbound-relations.md) - Accepted
* [0020 — Hand-write hook is scoped to CLI-owned type directories, not the whole bundle](0020-hand-write-hook-is-scoped-to-cli-owned-type-directories-not-the-whole-bundle.md) - Accepted

## Superseded

* [0006 — The web view is a read-only axum server reusing living-docs-core](0006-web-read-only-axum.md) - Superseded
* [0014 — The CLI serves skill content from an embedded corpus; harness SKILL.md files are slim stubs](0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md) - Superseded
* [0019 — Hand-written record frontmatter is blocked at write time, detected by check, and taught at point of use](0019-hand-written-record-frontmatter-is-blocked-at-write-time-detected-by-check-and-taught-at-point-of-use.md) - Superseded
