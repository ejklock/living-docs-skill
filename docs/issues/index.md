# Issues

Vertical, demoable slices for the living-docs system (greenfield backlog). Each issue
delivers one user-observable behavior end-to-end and links up the trail
([Constitution](../constitution.md) → [ADRs](../adr/)). Consume them smallest/lowest first,
one slice per fresh context, starting from the skeleton.

## Open

## Closed

* [0001 — Walking skeleton — Cargo workspace + living-docs-core + fs-store + thin cli](0001-workspace-core-skeleton.md) - done
* [0002 — Findability — db sync builds a SQLite/FTS5 read-model and living-docs search queries it](0002-findability-search.md) - done
* [0003 — Read-only web view — axum search + record page over the read-model](0003-web-read-only.md) - done
* [0004 — ParadeDB (Postgres + BM25) as a selectable db engine alongside SQLite](0004-paradedb-engine.md) - done
* [0005 — projects root + multi-project ingestion and cross-project search](0005-projects-multi-project.md) - done
* [0006 — db-mode authoritative authoring — new/index/supersede/check on db-store, lossless .md export](0006-db-mode-authoring.md) - done
* [0007 — Dev environment — docker-compose (ParadeDB) + Makefile targets over the workspace](0007-docker-compose-dev-env.md) - done
* [0008 — living-docs brief — deterministic pre-filled scaffold that leaves only the judgment slots for the authoring model](0008-brief-scaffold.md) - done
* [0009 — Per-type doc size targets — authoring convention in the skill corpus + advisory warning in check, plus the same-change economic rationale](0009-doc-size-targets.md) - done
* [0010 — Atlas create — db-mode authoring walking skeleton (mode guard, revision, transactional write+check)](0010-atlas-create-db-mode-authoring-walking-skeleton.md) - closed
* [0011 — Atlas edit — optimistic concurrency via a revision precondition](0011-atlas-edit-optimistic-concurrency-via-revision-precondition.md) - closed
* [0012 — Atlas supersede — browser parity with the CLI supersede verb](0012-atlas-supersede-browser-parity-with-the-cli-supersede-verb.md) - closed
* [0013 — Atlas delete — a new verb with no CLI precedent](0013-atlas-delete-a-new-verb-with-no-cli-precedent.md) - closed
* [0014 — --docs-dir is silently accepted but ignored by fmt and check, which operate on the cwd bundle](0014-docs-dir-is-silently-accepted-but-ignored-by-fmt-and-check-which-operate-on-the-cwd-bundle.md) - closed
* [0015 — status with a bare record number resolves across type directories, ADRs first](0015-status-with-a-bare-record-number-resolves-across-type-directories-adrs-first.md) - closed
* [0016 — check needs a ratchet or changed-files mode so brownfield repos can arm the pre-commit channel](0016-check-needs-a-ratchet-or-changed-files-mode-so-brownfield-repos-can-arm-the-pre-commit-channel.md) - closed
* [0017 — bundle vocabulary gaps: no research doc type in index and no terminal closed status for issues](0017-bundle-vocabulary-gaps-no-research-doc-type-in-index-and-no-terminal-closed-status-for-issues.md) - closed
* [0018 — Identity cannot express an author-named record in a directory, so glossary and the Context family stay hand-authored](0018-identity-cannot-express-an-author-named-record-in-a-directory-so-glossary-and-the-context-family-stay-hand-authored.md) - closed
* [0019 — The harness matrix is six targets where four are the same copy with a different destination](0019-the-harness-matrix-is-six-targets-where-four-are-the-same-copy-with-a-different-destination.md) - closed
* [0020 — The tracker status vocabulary disagrees across the template comment, the validator, and issue 0017 -- and has no in-progress state](0020-the-tracker-status-vocabulary-disagrees-across-the-template-comment-the-validator-and-issue-0017-and-has-no-in-progress-state.md) - closed
* [0021 — new has no --description flag, yet description is CLI-owned frontmatter](0021-new-has-no-description-flag-yet-description-is-cli-owned-frontmatter.md) - closed
* [0022 — Template placeholders are fragile for programmatic editing](0022-template-placeholders-are-fragile-for-programmatic-editing.md) - closed
* [0023 — next reports 0001 for issue because run_next passes the CLI token straight through instead of resolving its directory](0023-next-reports-0001-for-issue-because-run-next-passes-the-cli-token-straight-through-instead-of-resolving-its-directory.md) - closed
* [0024 — Doc-code pairing for living-docs: commit trailers, covers-based drift detection, and executable acceptance](0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) - closed
* [0025 — describe and status resolve record numbers ambiguously across doc-type directories](0025-describe-and-status-resolve-record-numbers-ambiguously-across-doc-type-directories.md) - closed
* [0026 — Corpus import: seed the knowledge graph read-model from real repos to make graph slices demoable from day one](0026-corpus-import-seed-the-knowledge-graph-read-model-from-real-repos-to-make-graph-slices-demoable-from-day-one.md) - closed
* [0027 — MCP front: expose the ten authoring verbs as MCP tools over the .md tree](0027-mcp-front-expose-the-ten-authoring-verbs-as-mcp-tools-over-the-md-tree.md) - closed
* [0028 — Responsibility split: one verb per module, sibling test files, and a hard file-size ratchet enforced by a deterministic check](0028-responsibility-split-one-verb-per-module-sibling-test-files-and-a-hard-file-size-ratchet-enforced-by-a-deterministic-check.md) - closed
* [0029 — status verb cannot set the issue lifecycle: number resolution prefers the ADR on cross-type collision and the status vocabulary is ADR-only](0029-status-verb-cannot-set-the-issue-lifecycle-number-resolution-prefers-the-adr-on-cross-type-collision-and-the-status-vocabulary-is-adr-only.md) - closed
* [0030 — db commands can create a literal 'sqlite:' directory by treating the connection string as a filesystem path](0030-db-commands-can-create-a-literal-sqlite-directory-by-treating-the-connection-string-as-a-filesystem-path.md) - closed
* [0031 — Bundle identity variant plus the artifact registry row and new artifact scaffolds a directory bundle with README](0031-bundle-identity-variant-plus-the-artifact-registry-row-and-new-artifact-scaffolds-a-directory-bundle-with-readme.md) - closed
* [0032 — check validates the artifact file manifest: missing listed file fails, unlisted orphan file warns](0032-check-validates-the-artifact-file-manifest-missing-listed-file-fails-unlisted-orphan-file-warns.md) - closed
* [0033 — index renders the Artifacts partition and the db-store projection indexes the artifact README body](0033-index-renders-the-artifacts-partition-and-the-db-store-projection-indexes-the-artifact-readme-body.md) - closed
* [0034 — Projection freshness SLA: search and web warn or refuse on a stale db-store projection](0034-projection-freshness-sla-search-and-web-warn-or-refuse-on-a-stale-db-store-projection.md) - closed
* [0035 — Provenance review queue: check flags records whose referenced source was superseded or changed status](0035-provenance-review-queue-check-flags-records-whose-referenced-source-was-superseded-or-changed-status.md) - closed
* [0036 — Owner field in record frontmatter, required for ADR and BDR by check](0036-owner-field-in-record-frontmatter-required-for-adr-and-bdr-by-check.md) - closed
* [0037 — Doc-readiness scorecard: a check subreport that grades a docs tree from human-era to agent-ready](0037-doc-readiness-scorecard-a-check-subreport-that-grades-a-docs-tree-from-human-era-to-agent-ready.md) - closed
* [0038 — db-mode struct round-trip of the owner field: db-store has no owner column and loads None](0038-db-mode-struct-round-trip-of-the-owner-field-db-store-has-no-owner-column-and-loads-none.md) - closed
* [0039 — moved-source clearing honors terminal statuses and self-supersession; the owner ratchet flips to require-owner](0039-moved-source-clearing-honors-terminal-statuses-and-self-supersession-the-owner-ratchet-flips-to-require-owner.md) - closed
* [0040 — the okf skill version tracks the vendored spec version, not the repo release](0040-the-okf-skill-version-tracks-the-vendored-spec-version-not-the-repo-release.md) - closed
* [0041 — fmt rewrites record bodies (reference lists collapse to one line) and supersede emits non-canonical frontmatter that sends users to fmt](0041-fmt-rewrites-record-bodies-reference-lists-collapse-to-one-line-and-supersede-emits-non-canonical-frontmatter-that-sends-users-to-fmt.md) - closed
* [0042 — Test suite duplication exceeds the jscpd five percent ratchet; extract shared helpers across db-store and command tests](0042-test-suite-duplication-exceeds-the-jscpd-five-percent-ratchet-extract-shared-helpers-across-db-store-and-command-tests.md) - closed
* [0043 — Retired-record callout: supersede/set/fmt write it, check enforces it, index rows name the successor, effective reports withheld count, skill rules teach the stop](0043-retired-record-callout-supersede-set-fmt-write-it-check-enforces-it-index-rows-name-the-successor-effective-reports-withheld-count-skill-rules-teach-the-stop.md) - closed
* [0044 — Execute the authoring-core cut: delete db-store, web, export, migrate, --json, seal remnants and the write-gate hook, retire their records and green the gates](0044-execute-the-authoring-core-cut-delete-db-store-web-export-migrate-json-seal-remnants-and-the-write-gate-hook-retire-their-records-and-green-the-gates.md) - closed
* [0045 — Three-pane web shell with metadata panel and Cmd+K palette](0045-three-pane-web-shell-with-metadata-panel-and-cmd-k-palette.md) - done
* [0046 — Packaging is the CLI: install.sh bootstraps the binary only, and the harness install matrix, the Claude Code plugin channel and the generated Copilot copy leave the repo](0046-packaging-is-the-cli-install-sh-bootstraps-the-binary-only-and-the-harness-install-matrix-the-claude-code-plugin-channel-and-the-generated-copilot-copy-leave-the-repo.md) - closed
* [0047 — set accepts title so a live record can be retitled without hand-editing frontmatter](0047-set-accepts-title-so-a-live-record-can-be-retitled-without-hand-editing-frontmatter.md) - closed
* [0048 — check stops advising on retired records and the live headings align with their titles](0048-check-stops-advising-on-retired-records-and-the-live-headings-align-with-their-titles.md) - closed
* [0049 — check liveness and moved-source reuse the shared retired-record predicate](0049-check-liveness-and-moved-source-reuse-the-shared-retired-record-predicate.md) - closed
* [0050 — jscpd ignores tests and markdown so the duplication gate measures production code](0050-jscpd-ignores-tests-and-markdown-so-the-duplication-gate-measures-production-code.md) - closed
* [0051 — check mermaid accepts diagrams that mermaid.js rejects](0051-check-mermaid-accepts-diagrams-that-mermaid-js-rejects.md) - closed
* [0052 — Section schema in the registry: check requires Scope and Acceptance on live issues and read gains a contract level](0052-section-schema-in-the-registry-check-requires-scope-and-acceptance-on-live-issues-and-read-gains-a-contract-level.md) - closed
* [0053 — Section heading parser counts indented-code headings and always drops the first heading as the title](0053-section-heading-parser-counts-indented-code-headings-and-always-drops-the-first-heading-as-the-title.md) - closed
* [0054 — Section schemas for ADR, PRD, constitution and research so check and read --contract cover every record type](0054-section-schemas-for-adr-prd-constitution-and-research-so-check-and-read-contract-cover-every-record-type.md) - closed
