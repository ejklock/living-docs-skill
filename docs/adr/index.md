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
* [0013 — Mermaid validation runs in-process via merman-core, not a Docker mermaid-cli shell-out](0013-mermaid-validation-runs-in-process-via-merman-core-not-a-docker-mermaid-cli-shell-out.md) - Accepted
* [0017 — SKILL.md stubs are pure routers; the spine and all detail move to CLI topics](0017-skill-md-stubs-are-pure-routers-the-spine-and-all-detail-move-to-cli-topics.md) - Accepted
* [0022 — Canonical frontmatter check is scoped to CLI-owned type directories](0022-canonical-frontmatter-check-is-scoped-to-cli-owned-type-directories.md) - Accepted
* [0025 — Releases are born draft and earn publication by passing the asset gate](0025-releases-are-born-draft-and-earn-publication-by-passing-the-asset-gate.md) - Accepted
* [0026 — A single DocType registry replaces nine hand-synced enumerations, and research and constitution enter as rows](0026-a-single-doctype-registry-replaces-nine-hand-synced-enumerations-and-research-and-constitution-enter-as-rows.md) - Accepted
* [0027 — Every rule keyed by doc type becomes a registry field, and glossary is not a doc type](0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md) - Accepted
* [0028 — The release binary is the unit of distribution: install.sh only bootstraps it and every placement becomes a CLI verb](0028-the-release-binary-is-the-unit-of-distribution-install-sh-only-bootstraps-it-and-every-placement-becomes-a-cli-verb.md) - Accepted
* [0029 — The status vocabulary is per doc type, sourced from one DocTypeSpec field -- not one global list validated against every template's own dialect](0029-the-status-vocabulary-is-per-doc-type-sourced-from-one-doctypespec-field-not-one-global-list-validated-against-every-template-s-own-dialect.md) - Accepted
* [0030 — Uniform brace markers for body placeholders](0030-uniform-brace-markers-for-body-placeholders.md) - Accepted
* [0033 — New consumers are fronts in the workspace, never new repos, until a deploy-cadence or ownership trigger fires](0033-new-consumers-are-fronts-in-the-workspace-never-new-repos-until-a-deploy-cadence-or-ownership-trigger-fires.md) - Accepted
* [0036 — Architecture views are a registry doc type on a Named identity with a kind-sequenced generated index](0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md) - Accepted
* [0041 — make cli-install fetches the released binary and install.sh resolves the latest release by default](0041-make-cli-install-fetches-the-released-binary-and-install-sh-resolves-the-latest-release-by-default.md) - Accepted
* [0043 — Owner is a CLI-owned frontmatter field with a warn-then-error ratchet on ADR and BDR](0043-owner-is-a-cli-owned-frontmatter-field-with-a-warn-then-error-ratchet-on-adr-and-bdr.md) - Accepted
* [0044 — Moved-source review queue: check emits a warn-level finding when a linked record is superseded or demoted](0044-moved-source-review-queue-check-emits-a-warn-level-finding-when-a-linked-record-is-superseded-or-demoted.md) - Accepted
* [0047 — living-docs fmt is frontmatter-only and the record body stays byte-identical](0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md) - Accepted
* [0048 — CLI mutation verbs emit canonical frontmatter so check never routes a user to fmt](0048-cli-mutation-verbs-emit-canonical-frontmatter-so-check-never-routes-a-user-to-fmt.md) - Accepted
* [0057 — Refocus living-docs on a decision log: delete BDR, cut peripheral verbs and check advisories, and consolidate the rule corpus](0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) - Accepted
* [0058 — Retired records declare themselves: a CLI-written callout in the body, successor-bearing index rows, and effective as the agent read verb](0058-retired-records-declare-themselves-a-cli-written-callout-in-the-body-successor-bearing-index-rows-and-effective-as-the-agent-read-verb.md) - Accepted
* [0059 — Cut living-docs to the authoring core: remove the database read-model, web front, public export, migrate, JSON authoring and the write-gate hook](0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) - Accepted
* [0060 — The CLI surface is a contract: ten intention-named verbs, one output mode per stream, documented exit codes, and help written for an agent](0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) - Accepted
* [0061 — set title retitles a record: frontmatter, heading, filename and every in-bundle link, refused only on a terminal or superseded record](0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md) - Accepted
* [0062 — check --changed-files scopes the gate to the records a commit touches, so a brownfield bundle can arm the hook](0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md) - Accepted
* [0063 — Authoring advisories skip retired records](0063-authoring-advisories-skip-retired-records.md) - Accepted
* [0064 — Each doc type declares its sections in the registry: required or optional, contract or detail tier, so check enforces the required ones on live records and read discloses by tier](0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md) - Accepted

## Superseded

_History only. Do not act on these records — run `living-docs read` for what is in force._

* [0003 — Storage backend is config-selected, mutually exclusive, and both modes authoritative](0003-storage-backend-model.md) - Deprecated (no successor)
* [0004 — db-mode runs on ParadeDB by default with SQLite opt-in, over SeaORM](0004-db-engine-and-data-layer.md) - Deprecated (no successor)
* [0005 — Normalized DB schema — projects root, typed records with an EAV tail, typed relations](0005-normalized-schema.md) - Deprecated (no successor)
* [0006 — The web view is a read-only axum server reusing living-docs-core](0006-web-read-only-axum.md) - Superseded by [0016](0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md)
* [0007 — db-mode authoring data model and lossless export contract](0007-db-mode-authoring-data-model-and-lossless-export-contract.md) - Deprecated (no successor)
* [0008 — BDR carries a required Contract section (public API + agent tool schemas)](0008-bdr-contract-section.md) - Deprecated (no successor)
* [0009 — Document visibility is default-deny frontmatter data, validated and index-aware](0009-document-visibility-model.md) - Deprecated (no successor)
* [0010 — Public export is a deterministic allowlist build with a leak gate; publish is a human-gated procedure](0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md) - Deprecated (no successor)
* [0011 — Secret and PII detection stays deterministic — a curated ruleset plus Shannon entropy, never ML](0011-leak-detection-stays-deterministic-curated-ruleset-plus-shannon-entropy-never-ml.md) - Deprecated (no successor)
* [0012 — Worldwide PII detection is checksum-tiered, two-stage, and deterministic](0012-worldwide-pii-detection-is-checksum-tiered-two-stage-and-deterministic.md) - Deprecated (no successor)
* [0014 — The CLI serves skill content from an embedded corpus; harness SKILL.md files are slim stubs](0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md) - Superseded by [0017](0017-skill-md-stubs-are-pure-routers-the-spine-and-all-detail-move-to-cli-topics.md)
* [0015 — Web UX follows the three-pane doc-site archetype with a search-first Cmd+K palette](0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md) - Deprecated (no successor)
* [0016 — Atlas makes the web a db-mode authoring front, superseding web read-only](0016-atlas-makes-the-web-a-db-mode-authoring-front-superseding-web-read-only.md) - Deprecated (no successor)
* [0018 — Atlas delete is a soft-delete, scoped to non-decision doc types, refused on inbound relations](0018-atlas-delete-is-a-soft-delete-scoped-to-non-decision-doc-types-refused-on-inbound-relations.md) - Deprecated (no successor)
* [0019 — Hand-written record frontmatter is blocked at write time, detected by check, and taught at point of use](0019-hand-written-record-frontmatter-is-blocked-at-write-time-detected-by-check-and-taught-at-point-of-use.md) - Superseded by [0020](0020-hand-write-hook-is-scoped-to-cli-owned-type-directories-not-the-whole-bundle.md)
* [0020 — Hand-write hook is scoped to CLI-owned type directories, not the whole bundle](0020-hand-write-hook-is-scoped-to-cli-owned-type-directories-not-the-whole-bundle.md) - Deprecated (no successor)
* [0021 — Enforcement layers ship with the repo: write-gate hook, session teaching, and pre-commit doc-gate](0021-enforcement-layers-ship-with-the-repo-write-gate-hook-session-teaching-and-pre-commit-doc-gate.md) - Deprecated (no successor)
* [0023 — Hooks ship through two deterministic channels: an in-repo Claude Code plugin and a living-docs hooks install verb](0023-hooks-ship-through-two-deterministic-channels-an-in-repo-claude-code-plugin-and-a-living-docs-hooks-install-verb.md) - Deprecated (no successor)
* [0024 — A release is atomic: a missing binary asset fails the workflow and demotes the release to a draft](0024-a-release-is-atomic-a-missing-binary-asset-fails-the-workflow-and-demotes-the-release-to-a-draft.md) - Superseded by [0025](0025-releases-are-born-draft-and-earn-publication-by-passing-the-asset-gate.md)
* [0031 — The knowledge graph is a typed bi-temporal edge set in the existing relational store](0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md) - Deprecated (no successor)
* [0032 — Doc-code lineage is declared, not inferred](0032-doc-code-lineage-is-declared-not-inferred.md) - Deprecated (no successor)
* [0034 — Artifacts are directory-bundle records: a canonical README plus a validated file manifest on a new Bundle identity](0034-artifacts-are-directory-bundle-records-a-canonical-readme-plus-a-validated-file-manifest-on-a-new-bundle-identity.md) - Deprecated (no successor)
* [0035 — Requirement IDs are PRD-scoped EARS statements and check traces BDR coverage](0035-requirement-ids-are-prd-scoped-ears-statements-and-check-traces-bdr-coverage.md) - Deprecated (no successor)
* [0037 — Migration is a deterministic advisor verb plus skill-guided judgment](0037-migration-is-a-deterministic-advisor-verb-plus-skill-guided-judgment.md) - Deprecated (no successor)
* [0038 — Record bodies are authorable as section-keyed JSON through new --json](0038-record-bodies-are-authorable-as-section-keyed-json-through-new-json.md) - Deprecated (no successor)
* [0039 — CLI-produced records carry an ephemeral HMAC provenance seal that check verifies](0039-cli-produced-records-carry-an-ephemeral-hmac-provenance-seal-that-check-verifies.md) - Deprecated (no successor)
* [0040 — migrate --apply is a CLI-front transaction over the mechanical subset](0040-migrate-apply-is-a-cli-front-transaction-over-the-mechanical-subset.md) - Deprecated (no successor)
* [0042 — Projection staleness contract: a sync_meta fingerprint row, checked by search against the records tree](0042-projection-staleness-contract-a-sync-meta-fingerprint-row-checked-by-search-against-the-records-tree.md) - Deprecated (no successor)
* [0045 — Doc-readiness scorecard is a read-only verb over existing check passes with a fixed attribute-signal table](0045-doc-readiness-scorecard-is-a-read-only-verb-over-existing-check-passes-with-a-fixed-attribute-signal-table.md) - Deprecated (no successor)
* [0046 — fmt unwraps hard-wrapped prose: one paragraph is one line, and the authoring rule says so](0046-fmt-unwraps-hard-wrapped-prose-one-paragraph-is-one-line-and-the-authoring-rule-says-so.md) - Superseded by [0047](0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md)
* [0049 — Mechanical record liveness: check derives stale-proposed and stale-impact from the linked issue and Verification paths](0049-mechanical-record-liveness-check-derives-stale-proposed-and-stale-impact-from-the-linked-issue-and-verification-paths.md) - Deprecated (no successor)
* [0050 — Effective view: a read verb compiling active records with supersede chains collapsed, progressive tiers, and a hard token budget](0050-effective-view-a-read-verb-compiling-active-records-with-supersede-chains-collapsed-progressive-tiers-and-a-hard-token-budget.md) - Deprecated (no successor)
* [0051 — why <path>: a reverse index from Implementation-impact lists so provenance is a query, not a code comment](0051-why-path-a-reverse-index-from-implementation-impact-lists-so-provenance-is-a-query-not-a-code-comment.md) - Deprecated (no successor)
* [0052 — Materiality criterion for the doc trail: a record earns an ADR/BDR when the decision is expensive to reverse, with advisory inflation signals](0052-materiality-criterion-for-the-doc-trail-a-record-earns-an-adr-bdr-when-the-decision-is-expensive-to-reverse-with-advisory-inflation-signals.md) - Deprecated (no successor)
* [0053 — Scorecard measures consumption, not only corpus shape: a fail-open capture hook feeds docs-tokens-read, stale-reads, and a table-driven doc-trail finding classifier](0053-scorecard-measures-consumption-not-only-corpus-shape-a-fail-open-capture-hook-feeds-docs-tokens-read-stale-reads-and-a-table-driven-doc-trail-finding-classifier.md) - Deprecated (no successor)
* [0054 — Teach record types by a decision test and a counterexample leak table, delivered in the template slot, with a LEAK advisory for the detectable cases](0054-teach-record-types-by-a-decision-test-and-a-counterexample-leak-table-delivered-in-the-template-slot-with-a-leak-advisory-for-the-detectable-cases.md) - Deprecated (no successor)
* [0055 — Semantic refusal triggers become instruments or advisory: DIAGRAM and DUPLICATE checks, and no prose-only hard stops](0055-semantic-refusal-triggers-become-instruments-or-advisory-diagram-and-duplicate-checks-and-no-prose-only-hard-stops.md) - Deprecated (no successor)
* [0056 — effective --topic ranks by the FTS5 read-model when a fresh projection exists, falling back to a deterministic relevance rank](0056-effective-topic-ranks-by-the-fts5-read-model-when-a-fresh-projection-exists-falling-back-to-a-deterministic-relevance-rank.md) - Deprecated (no successor)
