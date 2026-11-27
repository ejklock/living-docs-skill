# Issues

Vertical, demoable slices for the living-docs system (greenfield backlog). Each issue
delivers one user-observable behavior end-to-end and links up the trail
([Constitution](../constitution.md) → [ADRs](../adr/)). Consume them smallest/lowest first,
one slice per fresh context, starting from the skeleton.

## Open

* [0121 — read --all does not show which listed records are retired](0121-read-all-does-not-show-which-listed-records-are-retired.md) - open
* [0122 — Docs subagent on a cheaper model: exploration recipe now, drafting only after measured authoring cost](0122-docs-subagent-on-a-cheaper-model-exploration-recipe-now-drafting-only-after-measured-authoring-cost.md) - open
* [0126 — Implement ADR 0093: the opt-in decision prompt Stop hook and its installation](0126-implement-adr-0083-the-opt-in-decision-prompt-stop-hook-and-its-installation.md) - open
* [0132 — The HTML renderer duplicates the record reader and link graph of read records](0132-the-html-renderer-duplicates-the-record-reader-and-link-graph-of-read-records.md) - open
* [0144 — Engineering-docs roadmap: domain language and rules, decision capture, issue waves, generated diagrams and a local HTML export](0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md) - open
* [0152 — Formal proofs of state and decision artifacts, model-checker export, and bounded model checking of the CLI](0152-formal-proofs-of-state-and-decision-artifacts-model-checker-export-and-bounded-model-checking-of-the-cli.md) - open
* [0157 — The session-context tests read the host's git configuration](0157-the-session-context-tests-read-the-host-s-git-configuration.md) - open
* [0158 — manage fmt reports success on a bundle root that is a regular file](0158-manage-fmt-reports-success-on-a-bundle-root-that-is-a-regular-file.md) - open
* [0159 — The MCP check error on a missing bundle omits the CLI's prefix and hint](0159-the-mcp-check-error-on-a-missing-bundle-omits-the-cli-s-prefix-and-hint.md) - open
* [0160 — write new does not list the bundle root it wrote](0160-write-new-does-not-list-the-bundle-root-it-wrote.md) - open
* [0161 — Initialize-era MCP clients are offered resource subscriptions the server cannot serve them](0161-initialize-era-mcp-clients-are-offered-resource-subscriptions-the-server-cannot-serve-them.md) - open
* [0162 — Re-measure the MCP check task threshold on the CI runner](0162-re-measure-the-mcp-check-task-threshold-on-the-ci-runner.md) - open
* [0163 — Configure the manage html output path in the bundle root and refresh it from the pre-commit hook](0163-configure-the-manage-html-output-path-in-the-bundle-root-and-refresh-it-from-the-pre-commit-hook.md) - open
* [0164 — Give the generated HTML a sidebar navigation and a reading style](0164-give-the-generated-html-a-sidebar-navigation-and-a-reading-style.md) - open
* [0165 — Teach when each optional record type fits and advise on missing coverage in check](0165-teach-when-each-optional-record-type-fits-and-advise-on-missing-coverage-in-check.md) - open

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
* [0027 — MCP front: expose the ten authoring verbs as MCP tools over the .md tree](0027-mcp-front-expose-the-ten-authoring-verbs-as-mcp-tools-over-the-md-tree.md) - Superseded by [0111](0111-mcp-front-serve-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-living-docs-mcp.md)
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
* [0055 — new, set and supersede regenerate the touched index in the same run](0055-new-set-and-supersede-regenerate-the-touched-index-in-the-same-run.md) - closed
* [0056 — Help lists the record types, references accept the directory name and a record path, and a type error names the valid ones](0056-help-lists-the-record-types-references-accept-the-directory-name-and-a-record-path-and-a-type-error-names-the-valid-ones.md) - closed
* [0057 — new seeds owner from git config when --owner is absent and slugs transliterate accented letters](0057-new-seeds-owner-from-git-config-when-owner-is-absent-and-slugs-transliterate-accented-letters.md) - closed
* [0058 — check text output collapses advisories into one line per kind and lists them only with --advisories](0058-check-text-output-collapses-advisories-into-one-line-per-kind-and-lists-them-only-with-advisories.md) - closed
* [0059 — new --body and set --from read prose from a file or stdin](0059-new-body-and-set-from-read-prose-from-a-file-or-stdin.md) - closed
* [0060 — read --topic matches whole-word prefixes, ranks by weighted term frequency and says when nothing live matches](0060-read-topic-matches-whole-word-prefixes-ranks-by-weighted-term-frequency-and-says-when-nothing-live-matches.md) - closed
* [0061 — npm package living-docs wraps the release binary and the release workflow publishes it](0061-npm-package-living-docs-wraps-the-release-binary-and-the-release-workflow-publishes-it.md) - closed
* [0062 — The help documents every argument, vocabulary and reference form, and prints minified JSON off a TTY](0062-the-help-documents-every-argument-vocabulary-and-reference-form-and-prints-minified-json-off-a-tty.md) - closed
* [0063 — Implement duplicate-number detection and set number, and renumber this repository's open duplicates](0063-implement-duplicate-number-detection-and-set-number-and-renumber-this-repository-s-open-duplicates.md) - closed
* [0064 — Implement union merge of type indexes and the index-drift violation](0064-implement-union-merge-of-type-indexes-and-the-index-drift-violation.md) - closed
* [0065 — Implement fmt's repair of timestamp, owner and title heading](0065-implement-fmt-s-repair-of-timestamp-owner-and-title-heading.md) - closed
* [0066 — check accepts a record path and fails on a one-way supersede link, and supersede keeps an earlier supersedes value](0066-check-accepts-a-record-path-and-fails-on-a-one-way-supersede-link-and-supersede-keeps-an-earlier-supersedes-value.md) - closed
* [0067 — A fresh bundle passes check from its first record: the bundle-root index is written and new constitution uses its own vocabulary and title](0067-a-fresh-bundle-passes-check-from-its-first-record-the-bundle-root-index-is-written-and-new-constitution-uses-its-own-vocabulary-and-title.md) - closed
* [0068 — Remove the glossary and architecture-index templates and the code-comment-hygiene rule from the embedded corpus](0068-remove-the-glossary-and-architecture-index-templates-and-the-code-comment-hygiene-rule-from-the-embedded-corpus.md) - closed
* [0069 — Remove the effective and skill aliases and the guide --topic alias that ADR 0060 kept for one release](0069-remove-the-effective-and-skill-aliases-and-the-guide-topic-alias-that-adr-0060-kept-for-one-release.md) - closed
* [0070 — Teach the TYPE/NNNN reference where agents split it: the set usage error and the session hook line](0070-teach-the-type-nnnn-reference-where-agents-split-it-the-set-usage-error-and-the-session-hook-line.md) - closed
* [0071 — check suggests install hooks when the repository has no armed doc-gate](0071-check-suggests-install-hooks-when-the-repository-has-no-armed-doc-gate.md) - closed
* [0072 — fmt keeps a CRLF record's single frontmatter block and line endings](0072-fmt-keeps-a-crlf-record-s-single-frontmatter-block-and-line-endings.md) - closed
* [0073 — A record with no title gets one without its number and never from fenced text](0073-a-record-with-no-title-gets-one-without-its-number-and-never-from-fenced-text.md) - closed
* [0074 — set title rewrites only the title heading fmt and check judge](0074-set-title-rewrites-only-the-title-heading-fmt-and-check-judge.md) - closed
* [0075 — index reads a CRLF record's title and status](0075-index-reads-a-crlf-record-s-title-and-status.md) - closed
* [0076 — fmt keeps a quoted flow-list item quoted](0076-fmt-keeps-a-quoted-flow-list-item-quoted.md) - closed
* [0077 — The procedure and hard-rules guides say fmt unwraps hard-wrapped prose, but fmt is frontmatter-only](0077-the-procedure-and-hard-rules-guides-say-fmt-unwraps-hard-wrapped-prose-but-fmt-is-frontmatter-only.md) - closed
* [0078 — size-targets guide exempts architecture docs, but the registry gives the architecture view a size target](0078-size-targets-guide-exempts-architecture-docs-but-the-registry-gives-the-architecture-view-a-size-target.md) - closed
* [0079 — read lists closed and done issues although the guides say it shows active records only](0079-read-lists-closed-and-done-issues-although-the-guides-say-it-shows-active-records-only.md) - closed
* [0080 — index keeps a hand table header and a hand bullet list above the generated listing, leaving a dangling header or a duplicated list](0080-index-keeps-a-hand-table-header-and-a-hand-bullet-list-above-the-generated-listing-leaving-a-dangling-header-or-a-duplicated-list.md) - closed
* [0081 — set cannot write the tracker, labels and blocked_by keys that the issue workflow puts in issue frontmatter](0081-set-cannot-write-the-tracker-labels-and-blocked-by-keys-that-the-issue-workflow-puts-in-issue-frontmatter.md) - closed
* [0082 — set refuses architecture views, so a view's description is edited by hand](0082-set-refuses-architecture-views-so-a-view-s-description-is-edited-by-hand.md) - closed
* [0083 — The guides prescribe /-bundle-relative links, but index and supersede write sibling-relative links and check accepts every style](0083-the-guides-prescribe-bundle-relative-links-but-index-and-supersede-write-sibling-relative-links-and-check-accepts-every-style.md) - closed
* [0084 — fmt drops a tail key whose value is the empty string](0084-fmt-drops-a-tail-key-whose-value-is-the-empty-string.md) - closed
* [0085 — fmt rewrites the title heading of a record titled with digits only on every run](0085-fmt-rewrites-the-title-heading-of-a-record-titled-with-digits-only-on-every-run.md) - closed
* [0086 — the canonical writer does not escape a carriage return so such a value is refused](0086-the-canonical-writer-does-not-escape-a-carriage-return-so-such-a-value-is-refused.md) - closed
* [0087 — fmt rewrites a record whose frontmatter does not parse and drops its keys](0087-fmt-rewrites-a-record-whose-frontmatter-does-not-parse-and-drops-its-keys.md) - closed
* [0088 — a set read-back test writes a fixed-name temp file and fails when runs overlap](0088-a-set-read-back-test-writes-a-fixed-name-temp-file-and-fails-when-runs-overlap.md) - closed
* [0089 — six functions exceed the 30-line limit because the workspace clippy gate omits too_many_lines](0089-six-functions-exceed-the-30-line-limit-because-the-workspace-clippy-gate-omits-too-many-lines.md) - closed
* [0090 — an index row splits across two lines when a record title holds a line break](0090-an-index-row-splits-across-two-lines-when-a-record-title-holds-a-line-break.md) - closed
* [0091 — the file-size and allow-inventory gates fail inside a git worktree under .claude/worktrees](0091-the-file-size-and-allow-inventory-gates-fail-inside-a-git-worktree-under-claude-worktrees.md) - closed
* [0092 — index derives a row title from a fenced heading and leaves it empty without one](0092-index-derives-a-row-title-from-a-fenced-heading-and-leaves-it-empty-without-one.md) - closed
* [0093 — the unnumbered title-heading fallback may rename a section heading such as References](0093-the-unnumbered-title-heading-fallback-may-rename-a-section-heading-such-as-references.md) - closed
* [0094 — the one-way supersede link check lacks tests for its spelling, shared-number and path rules](0094-the-one-way-supersede-link-check-lacks-tests-for-its-spelling-shared-number-and-path-rules.md) - closed
* [0095 — check skips every entry of a supersedes list that mixes record numbers and other values](0095-check-skips-every-entry-of-a-supersedes-list-that-mixes-record-numbers-and-other-values.md) - closed
* [0096 — reserved files such as index.md are written outside the rooted store guard](0096-reserved-files-such-as-index-md-are-written-outside-the-rooted-store-guard.md) - closed
* [0097 — check and read grow quadratically with the bundle size](0097-check-and-read-grow-quadratically-with-the-bundle-size.md) - closed
* [0098 — read --links-to and --impact list the records that depend on a record](0098-read-links-to-and-impact-list-the-records-that-depend-on-a-record.md) - closed
* [0099 — data verbs panic when the reader closes stdout](0099-data-verbs-panic-when-the-reader-closes-stdout.md) - closed
* [0100 — integration tests leave their scratch directories behind](0100-integration-tests-leave-their-scratch-directories-behind.md) - closed
* [0101 — core keeps dead run functions that print to stdout](0101-core-keeps-dead-run-functions-that-print-to-stdout.md) - closed
* [0102 — core keeps dead set and supersede run functions](0102-core-keeps-dead-set-and-supersede-run-functions.md) - closed
* [0103 — integration tests keep redundant manual cleanup calls](0103-integration-tests-keep-redundant-manual-cleanup-calls.md) - closed
* [0104 — check module docblock links to a run function that no longer exists](0104-check-module-docblock-links-to-a-run-function-that-no-longer-exists.md) - closed
* [0105 — check accepts the placeholder description that new writes](0105-check-accepts-the-placeholder-description-that-new-writes.md) - closed
* [0106 — Project guide is AGENTS.md with CLAUDE.md a symlink to it](0106-project-guide-is-agents-md-with-claude-md-a-symlink-to-it.md) - closed
* [0107 — Ship the living-docs-drift skill: anchor resolution plus a configurable LLM judge, advisory report outside the bundle](0107-ship-the-living-docs-drift-skill-anchor-resolution-plus-a-configurable-llm-judge-advisory-report-outside-the-bundle.md) - closed
* [0108 — A skill with no topics lists as an empty line in guide, and the project guide miscounts the verbs](0108-a-skill-with-no-topics-lists-as-an-empty-line-in-guide-and-the-project-guide-miscounts-the-verbs.md) - closed
* [0109 — Regroup the CLI under write, read and manage with check top-level, and move every surface that teaches a command](0109-regroup-the-cli-under-write-read-and-manage-with-check-top-level-and-move-every-surface-that-teaches-a-command.md) - closed
* [0110 — Publish the npm package under @ejklock/living-docs](0110-publish-the-npm-package-under-ejklock-living-docs.md) - closed
* [0111 — MCP front: serve the bundle over stdio as write, read, manage and check tools from living-docs mcp](0111-mcp-front-serve-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-living-docs-mcp.md) - closed
* [0112 — stale-proposed counts only issues that declare Implements for the ADR](0112-stale-proposed-counts-only-issues-that-declare-implements-for-the-adr.md) - closed
* [0113 — write set section refuses to drop child headings](0113-write-set-section-refuses-to-drop-child-headings.md) - closed
* [0114 — The distributed pre-commit hook lacks the changed-only opt-in the repository hook has](0114-the-distributed-pre-commit-hook-lacks-the-changed-only-opt-in-the-repository-hook-has.md) - closed
* [0115 — The changed-only pre-commit tests inherit the host's git config and environment](0115-the-changed-only-pre-commit-tests-inherit-the-host-s-git-config-and-environment.md) - closed
* [0116 — The file-size ratchet reports a stale baseline inside a fleet worktree](0116-the-file-size-ratchet-reports-a-stale-baseline-inside-a-fleet-worktree.md) - closed
* [0117 — The liveness declaration scan walks links with its own parser instead of link_destinations](0117-the-liveness-declaration-scan-walks-links-with-its-own-parser-instead-of-link-destinations.md) - closed
* [0118 — The size gates pass when find fails to read part of the tree](0118-the-size-gates-pass-when-find-fails-to-read-part-of-the-tree.md) - closed
* [0119 — The doc-trail gate reads a fixture path in a test as a record citation](0119-the-doc-trail-gate-reads-a-fixture-path-in-a-test-as-a-record-citation.md) - closed
* [0120 — Two parity test helpers exceed the function line cap and fail clippy](0120-two-parity-test-helpers-exceed-the-function-line-cap-and-fail-clippy.md) - closed
* [0123 — MCP discovery instructions lack the mid-work decision teaching the session line gained](0123-mcp-discovery-instructions-lack-the-mid-work-decision-teaching-the-session-line-gained.md) - closed
* [0124 — session-context.sh header comment understates what the session line teaches](0124-session-context-sh-header-comment-understates-what-the-session-line-teaches.md) - closed
* [0125 — Implement ADR 0092: blocked_by validation in check, write-time refusal and renumber rewrite](0125-implement-adr-0082-blocked-by-validation-in-check-write-time-refusal-and-renumber-rewrite.md) - closed
* [0127 — Implement ADR 0090 and 0091: manage html renderer, manage install mermaid and the CLI wiring](0127-implement-adr-0084-and-0085-manage-html-renderer-manage-install-mermaid-and-the-cli-wiring.md) - closed
* [0128 — A 23-digit blocked_by item is refused with a message naming the type key](0128-a-23-digit-blocked-by-item-is-refused-with-a-message-naming-the-type-key.md) - closed
* [0129 — manage install mermaid has no end-to-end test of its success path](0129-manage-install-mermaid-has-no-end-to-end-test-of-its-success-path.md) - closed
* [0130 — Renumbering renames the record before rewriting references, so a failed rewrite leaves the bundle half-renumbered](0130-renumbering-renames-the-record-before-rewriting-references-so-a-failed-rewrite-leaves-the-bundle-half-renumbered.md) - closed
* [0131 — The canonical writer leaves zero-padded numeric strings unquoted](0131-the-canonical-writer-leaves-zero-padded-numeric-strings-unquoted.md) - closed
* [0133 — Implement ADR 0086 and 0087: blocked_by advice for free-form items and a once-per-commit decision prompt](0133-implement-adr-0086-and-0087-blocked-by-advice-for-free-form-items-and-a-once-per-commit-decision-prompt.md) - closed
* [0134 — The check rule lists only some advisory kinds](0134-the-check-rule-lists-only-some-advisory-kinds.md) - closed
* [0135 — blocked_by cannot hold an item YAML reads as another type](0135-blocked-by-cannot-hold-an-item-yaml-reads-as-another-type.md) - closed
* [0136 — A blocked_by item beyond the u64 range is refused with a message naming the wrong key](0136-a-blocked-by-item-beyond-the-u64-range-is-refused-with-a-message-naming-the-wrong-key.md) - closed
* [0137 — write set reads a value that starts with a dash as a flag](0137-write-set-reads-a-value-that-starts-with-a-dash-as-a-flag.md) - closed
* [0138 — Implement ADR 0088 and 0089: blocked_by findings as advice and the decision reminder as a non-blocking nudge](0138-implement-adr-0088-and-0089-blocked-by-findings-as-advice-and-the-decision-reminder-as-a-non-blocking-nudge.md) - closed
* [0139 — The check summary repeats a file's number once per advisory](0139-the-check-summary-repeats-a-file-s-number-once-per-advisory.md) - closed
* [0140 — The decision prompt goes silent when the working directory path holds a quote](0140-the-decision-prompt-goes-silent-when-the-working-directory-path-holds-a-quote.md) - closed
* [0141 — check repeats the arm-the-hooks hint on every run](0141-check-repeats-the-arm-the-hooks-hint-on-every-run.md) - closed
* [0142 — The pre-commit gate prints the bundle's absolute path](0142-the-pre-commit-gate-prints-the-bundle-s-absolute-path.md) - closed
* [0143 — The HTML page keeps empty group headings under a filter and repeats a view's title](0143-the-html-page-keeps-empty-group-headings-under-a-filter-and-repeats-a-view-s-title.md) - closed
* [0145 — A failing callout reconcile after a renumber leaves the bundle renamed](0145-a-failing-callout-reconcile-after-a-renumber-leaves-the-bundle-renamed.md) - closed
* [0146 — The changed-only pre-commit drops staged files whose names hold a space](0146-the-changed-only-pre-commit-drops-staged-files-whose-names-hold-a-space.md) - closed
* [0147 — write set refuses a purely numeric label](0147-write-set-refuses-a-purely-numeric-label.md) - closed
* [0148 — manage index writes a link to a filename with a space that check cannot resolve](0148-manage-index-writes-a-link-to-a-filename-with-a-space-that-check-cannot-resolve.md) - closed
* [0149 — Make the CLI deterministic across releases and close the paths that leave a bundle red or silently wrong](0149-make-the-cli-deterministic-across-releases-and-close-the-paths-that-leave-a-bundle-red-or-silently-wrong.md) - closed
* [0150 — Domain records in docs/context nested by folder, and anchor-checked links](0150-domain-records-in-docs-context-nested-by-folder-and-anchor-checked-links.md) - closed
* [0151 — Living diagrams: stamped generated view regions, splice and stale-view](0151-living-diagrams-stamped-generated-view-regions-splice-and-stale-view.md) - closed
* [0153 — Measure how often a stale or contradicted decision is caught as the log grows past 60 records](0153-measure-how-often-a-stale-or-contradicted-decision-is-caught-as-the-log-grows-past-60-records.md) - closed
* [0154 — Make the record lifecycle an explicit transition table and refuse illegal supersede transitions](0154-make-the-record-lifecycle-an-explicit-transition-table-and-refuse-illegal-supersede-transitions.md) - closed
* [0156 — Ship living-docs as a Claude Code plugin from a marketplace at the repository root](0156-ship-living-docs-as-a-claude-code-plugin-from-a-marketplace-at-the-repository-root.md) - closed
