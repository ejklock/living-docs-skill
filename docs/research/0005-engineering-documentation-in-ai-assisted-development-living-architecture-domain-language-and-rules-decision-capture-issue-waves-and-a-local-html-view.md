---
type: Research
title: "Engineering documentation in AI-assisted development: living architecture, domain language and rules, decision capture, issue waves and a local HTML view"
description: Four parallel research sessions and one advisor review, 2026-11-15, behind the roadmap in issue 0144
status: Draft
timestamp: 2026-11-15T14:51:36Z
---

# 0005. Engineering documentation in AI-assisted development: living architecture, domain language and rules, decision capture, issue waves and a local HTML view

## Question

The owner asked five questions on 2026-11-15, after observing that one issue record concentrates terms, rules, decisions and work at once:

1. Where do the ubiquitous language and the business rules live, outside the issue, without bringing back the ADR+BDR pair inflation that [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) removed?
2. How does an agent record a decision at the moment it makes it mid-development, not after the fact?
3. How are issues grouped into waves: what can start in parallel, and what ships together?
4. Can architecture views, including the classic UML set, be generated from code and kept fresh by a deterministic gate?
5. Can the bundle be read as a browsable HTML site without repeating the periphery cost [ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) cut?

The owner framed the answer as a layered model: ubiquitous language → business rules → decisions → work → structure. This note tests that model against the literature and against the bundle's current homes.

## Method

- Four research agents ran in parallel on 2026-11-15, each read-only on the repository: (A) living architecture diagrams and UML; (B) business rules, ubiquitous language and issue waves; (C) documentation in AI-assisted engineering 2024-2026; (D) an HTML view of the bundle. One advisor then reviewed their synthesis; its judgment is recorded in Implications and labelled `[inference]`.
- Repository premises read by the agents: [ADR 0027](/adr/0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md), [ADR 0031](/adr/0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md), [ADR 0032](/adr/0032-doc-code-lineage-is-declared-not-inferred.md), [ADR 0033](/adr/0033-new-consumers-are-fronts-in-the-workspace-never-new-repos-until-a-deploy-cadence-or-ownership-trigger-fires.md), [ADR 0036](/adr/0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md), ADR 0057, ADR 0059, [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md), [ADR 0064](/adr/0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md), [ADR 0070](/adr/0070-type-indexes-merge-with-the-git-union-driver-and-check-fails-on-an-index-that-differs-from-its-regeneration.md), [ADR 0074](/adr/0074-read-links-to-and-impact-answer-who-depends-on-a-record-from-a-link-graph-built-in-memory-per-run.md), [ADR 0075](/adr/0075-doc-to-code-drift-is-an-advisory-skill-whose-llm-judge-runs-outside-the-binary-through-a-configurable-command.md), [ADR 0076](/adr/0076-the-cli-groups-its-operations-under-write-read-and-manage-as-nested-subcommands-with-check-kept-top-level.md), [ADR 0078](/adr/0078-an-mcp-front-serves-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-the-living-docs-mcp-subcommand.md), [ADR 0081](/adr/0081-the-distributed-pre-commit-hook-is-the-only-copy-and-environment-variables-opt-it-into-changed-only-and-owner-checks.md); [issue 0018](/issues/0018-identity-cannot-express-an-author-named-record-in-a-directory-so-glossary-and-the-context-family-stay-hand-authored.md), [issue 0024](/issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md), [issue 0068](/issues/0068-remove-the-glossary-and-architecture-index-templates-and-the-code-comment-hygiene-rule-from-the-embedded-corpus.md), [issue 0081](/issues/0081-set-cannot-write-the-tracker-labels-and-blocked-by-keys-that-the-issue-workflow-puts-in-issue-frontmatter.md); [the constitution](/constitution.md); [research 0004](/research/0004-model-based-architecture-views-distilled-from-the-knowledge-graph.md); `skills/living-docs/rules/architecture-diagrams.md`; `living-docs check --help`; commit `63536d8`.
- Repository facts verified by the session itself: `Identity::Named` is defined at `living-docs-core/src/doc_type.rs:20`; `Cargo.lock` lists 235 packages.
- Tool metadata (license, last push, last release) came from `gh api repos/<owner>/<repo>` and its `/releases/latest` or `/tags`, run 2026-11-15 by agent A; Mermaid diagram status came from `gh api` over `packages/mermaid/src/docs/syntax/` in `mermaid-js/mermaid`.
- The advisor ran `living-docs check --mermaid-only` on 0.28.0 against sample diagrams of each candidate syntax.
- Fetch gaps and items the reports did not verify: jdeps, madge, pydeps and emerge output formats came from prior knowledge, not refetched; pyreverse, cog, mermerd and prisma-erd-generator were read from raw GitHub files whose URLs the report did not record, so claims about them carry no reference below; SchemaCrawler's Mermaid templates and the Doctrine diagram bundle were not verified (low); Mermaid 12's change of default layout engine came from search summaries only; the recovery-technique scores in Garcia et al. and Lutellier et al. were not read (medium); SBVR, DMN, critical path method, Jira fix versions and BMAD storage were described from general knowledge (low); Kiro's task-to-requirement link and its "waves" grouping appear only on third-party blogs; the Thoughtworks Radar "agent instruction bloat" entry was read through a search summary (medium); Kiro hooks detail came from third-party guides; Starlight's Mermaid status, Zola's backlink fields, Pagefind's byte determinism, the size of `mermaid.min.js` and the Backstage 1.56 engine claim were not verified.
- The "47% of links never created versus 12% broken" figure is quoted from [research 0003](/research/0003-provenance-aware-knowledge-graph-for-living-docs.md) and its underlying study was not re-verified.
- The arXiv papers dated after mid-2026 that report C relies on (2606.09090, 2607.27250, 2607.21832, 2608.11095, 2609.03721, 2610.27381, and version 3 of 2602.11988) were not independently re-checked by the session; their figures carry the report's confidence, no higher.

## Findings

### Living architecture and UML

- Model-as-source tools solve cross-view drift, not doc-to-code drift: Structurizr exports PlantUML, C4-PlantUML, Mermaid, static sites and JSON but not DOT [1]; LikeC4 generates Mermaid, PlantUML, D2, DOT, JSON, draw.io and Markdown [4]. High.
- Only Structurizr's Java component finder derives a model from code, and only components inside a container [2][3]. LikeC4 is authored only; code-to-model exists as an unmerged pull request [5], a third-party agent tool [6] and a commercial product [7] `[COI: Archyl]`. Medium.
- The `structurizr/cli` repository is archived (last release v2025.11.09) and work continues in a monorepo; confidence on today's CLI packaging is medium [1].
- Mermaid C4 covers context, container, component, dynamic and deployment, but its layout depends on statement order and has no layout hints [8]. High.
- Context and container levels encode intent and stay authored; component and module levels can be derived from the import graph given a declared path-to-component mapping `[inference]`, high.
- Extractors with native Mermaid output: dependency-cruiser [10] and deptrac [12]; most others emit DOT, including cargo-modules for Rust [9]. High. A consumer of their output must canonicalize node and edge order `[inference]`.
- Symbol-level indexes are heavy or fading: stack-graphs is archived [15], LSIF is superseded by SCIP [16], and `rust-analyzer scip` skips `include!` files [17]. Import-level graphs suffice for module views `[inference]`.
- Embedding tree-sitter grammars in the binary would bring back the dependency growth ADR 0059 cut; extraction stays outside the binary `[inference]`, high.
- Schema to ERD: tbls reads many engines or a committed JSON snapshot, emits Mermaid and has a CI `diff` command (MIT) [18]; Atlas emits Mermaid text, but its default binary ships under a proprietary licence and its hosted ERD is a paid tier [19][20] `[COI: Ariga]`. High.
- The offline ERD path is: apply migrations to an ephemeral SQLite database, inspect, and commit a schema snapshot from which the ERD is rendered `[inference]`, medium-high.
- Readable generated diagrams follow one deterministic pipeline: lift edges to the view's granularity by a declared mapping; detect cycles with Tarjan SCC [21]; optionally apply transitive reduction [22], off by default for dependency views because it hides real coupling `[inference]`; leave Sugiyama layout to the renderer and emit sorted text `[inference]`.
- Clustering without a mapping: Bunch/MQ is search-based and non-deterministic unless seeded [23][24] and its robustness was questioned [25]; ACDC is pattern-driven and deterministic [26]; in a ten-technique comparison ACDC and ARC came closest to expert ground truth [27] (scores not read, medium). Clustering stays out of the default path `[inference]`.
- The reflexion model classifies each declared edge as convergence, divergence or absence from a high-level model, a source model and a mapping [28]. High. It is the deterministic structural counterpart of ADR 0075's LLM judge `[inference]`.
- Views as executable rules have precedent: ArchUnit checks code against a PlantUML component diagram and freezes known violations [14]; deptrac renders its layer rules as Mermaid [11][12]; dependency-cruiser keeps a baseline of known violations [10]; import-linter checks layer contracts without rendering [13]. High.
- Mermaid ships `classDiagram`, `sequenceDiagram`, `stateDiagram`, `erDiagram`, `c4`, `flowchart`, `architecture-beta` and `usecase-beta` (since 12.0.0), and has no UML object, activity, timing or UML-notation component diagram (`gh api` on the Mermaid docs tree); PlantUML covers the full UML set [29]. High.
- Rust-native Mermaid renderers exist (merman, mermaid-rs-renderer) but are young [30][31]. Low-medium on production readiness.
- Derivability by diagram: class from static types; package and component from imports plus a mapping; ER from schema; state machines from declarative FSM libraries that export Mermaid [32][33]; sequence from committed runtime traces with young tools [34][35] (low); use case, object and activity stay authored `[inference]`.
- ADR 0036's `kind` vocabulary covers C4, sequence, state, flow and data-model; class and use case are missing; package folds into `component` `[inference]`.
- `docs/architecture/` does not exist in this repository (`ls docs/architecture` fails).

### Ubiquitous language and business rules

- The Business Rules Manifesto holds that rules are first-class requirements, separate from process, stated declaratively [36]. EARS gives five sentence templates [37]. Gherkin's `Rule:` keyword groups the scenarios that illustrate one rule, and its home is the test suite [38]. High.
- Event Storming policies take the form "whenever event, then command" [39][40] (secondary sources, medium).
- The Bounded Context Canvas keeps "Ubiquitous Language" and "Business Decisions" side by side, per context [41]; arc42 tells authors to combine domain models with the glossary [42]. High.
- Ubiquitous language is per bounded context, because a term changes meaning when the model changes [53][54]. High. A global glossary is the wrong shape `[inference]`.
- Traceability tools keep rules addressable in git: Doorstop [43], StrictDoc with a stable machine ID [44], OpenFastTrace with revisioned IDs that mark stale coverage "Outdated" [45], sphinx-needs with ID regex validation [46]. High on mechanism; their fit to a small project is medium `[inference]`, since they target regulated domains.
- OpenSpec keeps a project-wide current-state spec in `openspec/specs/` and merges per-change deltas into it on archive [47][48]. High. It is the closest analogue to a living rules home separated from work `[inference]`.
- Kiro [49][50] `[COI: AWS]` and Spec Kit [51][52] `[COI: GitHub]` keep specs per feature, which drift after delivery, the same failure the owner sees in issues `[inference]`, medium.
- Context Mapper models contexts but holds no glossary [55]. Vale's vocabularies already enforce term spelling and flag avoided terms deterministically [56]; reuse beats rebuilding `[inference]`, high.
- ADR 0027's structural reason against a glossary row no longer holds: `Identity::Named` shipped with ADR 0036 (issue 0018). What still argues against it is issue 0068's usage audit and the ADR 0057/0059 scope discipline `[inference]`, high.
- `skills/living-docs/templates/context-index.md` and `rules/semantic-index.md` remain in the corpus after issue 0068.
- Report B ranked a `domain` record on `Identity::Named`, one file per bounded context with Terms and Rules sections edited in place, above a PRD section, Gherkin-only rules or the status quo `[inference]`, medium.
- A rule-to-test orphan report would re-open the traceability pass ADR 0057 removed.

### Issue waves

- Kahn's algorithm peels indegree-zero nodes in rounds, so successive rounds are parallel batches and leftover edges prove a cycle [57]. High.
- Grouping practices in use: Shape Up's fixed-time appetite [58][59] `[COI: Basecamp, mild]`; story-map release slices with a walking skeleton first [60]; GitHub milestones and blocked-by dependencies [61][62] `[COI: GitHub]`; Linear cycles that roll over and are not tied to releases [63] `[COI: Linear]`; Now/Next/Later horizons [64] `[COI: ProdPad, inventor's claim]`.
- Critical path method needs duration estimates the bundle does not hold `[inference]`, high.
- Spec Kit marks a task parallel only when it has no dependency and touches different files [51]. High. No surveyed tool computes file overlap from a spec; an author or agent declares it `[inference]`, high.
- `blocked_by` is free text today: `set` writes it on issues (issue 0081) and `grep -rn blocked_by living-docs-core/src/check` returns nothing, so no dangling, self-reference or cycle check exists. A consumer bundle (relent) carries 198 issues with `blocked_by`, per issue 0081.
- Report B ranked validating `blocked_by` first, a derived wave layering as a `read records` flag second, and an epic record type as rejected because it goes stale by construction `[inference]`, medium-high.

### Documentation in AI-assisted engineering

- Every surveyed approach keeps its artifacts as Markdown in the repository: Spec Kit [52], Kiro [50], OpenSpec [47], BMAD [68], AGENTS.md [69][70], Claude Code memory and skills [71]. High. This matches ADR 0059 `[inference]`.
- Tessl moved from spec-as-source to a skills package manager with evaluations [65][66][67] `[COI: Tessl]`; its "2x agent success" figure is a vendor claim (low).
- AGENTS.md moved to the Linux Foundation's Agentic AI Foundation on 2025-12-09 [69][70]; the adoption count differs across sources (60,000 versus 40,000) `[COI: OpenAI]`, medium.
- Convergence: a small always-loaded constitution with on-demand detail [72][73][78][79] `[COI: Anthropic]` for [72]; current truth kept apart from change history [47]; instructions moving into deterministic hooks and checks [79][71][81][82]. Medium-high.
- llms.txt shows no retrieval use: 97% of files got no requests in May 2026 [74]. Medium (SEO sources).
- Böckeler's taxonomy separates spec-first, spec-anchored and spec-as-source, and reports Kiro turning a small bug into 4 stories and 16 acceptance criteria, with instructions often not followed [83]. High. Few tools ship a CI gate on spec-code agreement [84] (medium).
- Thoughtworks Radar places spec-driven development in Assess, OpenSpec in Trial, and "AI-driven documentation sprawl" and "agent instruction bloat" in Caution [79][80] `[COI: Thoughtworks, consulting only]`. Medium-high.
- Effect of context files on correctness: Gloaguen et al. found no general improvement on SWE-bench-style tasks and over 20% more inference cost, with repository overviews unhelpful [85]; Khatri found no measurable correctness effect over 288 runs [86]. High in the report.
- Contradiction on cost: Lulla et al. measured median runtime −28.6% and output tokens −16.6% at comparable completion over 124 PRs [87], against Gloaguen's +20% [85]. Different tasks and metrics; Khatri attributes the gap to task selection [86]. Treat "context files help" as unproven either way. Medium.
- Instruction layout has no detectable effect on adherence, and adherence decays within a session (odds ratio 0.944 per generated function, post hoc) [88]. Medium, single author.
- LLM-generated context files help only when all other documentation is removed (+2.7%) [85]. High. Generated docs are mostly redundant with what the agent can read `[inference]`, medium.
- Agent-authored documentation PRs are accepted more often than human ones (Codex 88.6%, Claude Code 85.7%, human 76.5%); acceptance is not quality [89]. High.
- Drift before LLMs: up-to-dateness is a top documentation issue [90]; most of 3,000+ projects had an outdated code reference at some point [91], and a per-PR action exists [92]; just-in-time detection at commit time is feasible [93]. High.
- Ownership drives freshness: "Documents without owners become stale" [77]; a 90-day review ratchet found real drift but turned every branch red on day 91 [94]. Medium.
- Drift in the LLM era: 23.0% of 356 repositories had stale code references in AI configuration files [95]; 63.2% of 29,669 AI pull requests change code without touching any doc, and undocumented files are followed by a fix 8.6–11.7% more often [96] (correlational; causal reading low); instruction files grow +226% over their life because rules without rationale are unsafe to delete [97] (medium, single author); agent context files evolve by small additions and rarely state security or performance rules [98]. High on prevalence.
- Input length degrades all 18 models tested, most with similar distractors [99]. High `[COI: Chroma, claim not in its favour]`.
- LLMs draft ADR Decision sections below human level [100]; decisions mined from commits describe what changed, not why [101]; violation detection works for explicit, code-checkable ADRs only [102]; agents make implicit architectural choices that call for decision records [103] (position paper); ADR adoption in open source is low but rising [104]; LLM inconsistency detectors are volatile under paraphrase [105]. Multi-agent docstring generation beats baselines [106]. High except [104][105] medium.
- Rationale must be captured while the reasoning is in context, at end of turn or at commit, not by later mining `[inference]`, high, from [101][103].
- Capture mechanisms surveyed: always-loaded rules decay [88][83]; Claude Code, Codex and Cursor all ship a blockable stop event [71][81][82], so a Stop hook can fire on a deterministic trigger (diff ∩ paths, no record touched, one continuation) while rationale is still in context; a commit trailer mirrors `Co-Authored-By`, which a study used to identify agent PRs [109]; Danger's co-change rule warns rather than fails because "there are a lot of legitimate reasons" not to need the entry [107]; PR templates show mixed effects [108].
- No controlled study measures any hook or gate's effect on decision capture by agents (searched 2026-11-15). Medium, absence of evidence.
- Gates check the form and references of a declaration; the model supplies the judgment; an explicit "no decision" path is required or the gate becomes an inflation engine `[inference]`, high.
- Repository premises: `check --changed-files` narrows findings to record paths and does not map code paths to records (`living-docs check --help`); the only session hook is `skills/living-docs/hooks/session-context.sh`, with no Stop hook; issue 0024's commit trailers closed obsolete, ADR 0032 is Deprecated, and ADR 0057 removed the path-only stale-impact advisory.
- Prior ADR tooling: adr-tools [75] and Log4brains [76].

### HTML view of the bundle

- ADR 0059 removed the web front (2.3k lines), the read-model (about 5.7k lines) and public export (5k lines) because nobody in the authoring loop called them, they weighed two thirds of 352 locked dependencies, and flagged code rots. Commit `63536d8` removed 79 files and 15,935 lines, including `axum`, `tokio`, `maud` and `sea-orm`.
- The constitution brings a web front back only for cross-project search or an independently deployed surface, and keeps the binary free of host tools, daemons and databases ([constitution](/constitution.md), lines 30 and 54).
- Cheap precedents exist: ADR 0074's in-memory link graph, ADR 0078's front returning for a named consumer, `pulldown-cmark =0.13.4` already in core with HTML off, and `merman-core` for in-process Mermaid validation.
- A single-file static export can be byte-reproducible: reuse the `read` ordering, write no build timestamp, embed pinned assets by content, keep anchors stable; "the export twice gives identical bytes" becomes the fitness function `[inference]`, high.
- Mermaid in the page: vendored `mermaid.js`, a CDN script, or build-time SVG through merman, whose default features pull an EPL-2.0 layout engine and whose text measurement can differ from browsers [30]. High.
- Search: Pagefind 1.6.0 is a Rust library but runs on `tokio` and fetches chunks, so it fails from `file://` [110][111][112] `[COI: CloudCannon]`; an embedded JSON index with MiniSearch works from `file://` [113]. Medium-high.
- External generators are moving targets: Material for MkDocs entered maintenance mode on 2025-11-05 [114][115] with Zensical as successor [116] (secondary, COI of its author); Backstage TechDocs is leaving MkDocs [117]; mdBook 0.5 broke preprocessors [120]; Zola has native backlinks for `@/` links but 0.23 broke templates [121][122]; Quartz has native backlinks and a graph view but needs Node and a v5 migration [123][124]; Docusaurus has an official Mermaid theme [118]; Starlight's Mermaid support was not found [119] (low). Medium-high.
- ADR-only viewers cover one record type of six and are inactive: Log4brains last released 2024-12 [125], adr-viewer about 2024-12 [126]. High.
- A local static export avoids every cost ADR 0059 named only if it persists nothing, runs no server, adds no async or database dependency, reuses `read` and the link graph, and names its reader `[inference]`, high. Publishing it would reopen the leak question of Deprecated [ADR 0010](/adr/0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md).
- Report D estimated 600 to 1,000 lines with tests for a `manage` export and recommended no `serve` operation `[inference]`, medium.

## Implications

The owner's decisions on these findings live in the Decision section of [issue 0144](/issues/0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md); this note carries only the evidence and the critiques below.

- The layered model holds as a map of homes, not a pipeline: the literature places terms and rules together per bounded context [41][53], decisions in append-only records, and work apart from current truth [47] `[inference]`.
- Advisor's judgment `[inference]`: an in-binary graph snapshot with a region-equality check proves the region equals the snapshot, not that the snapshot equals the code, so the gate can be green over a stale diagram. The extractors already emit Mermaid [10][12], so the first step is a skill recipe plus a CI regenerate-and-diff [18]. A reflexion advisory is the class ADR 0057 removed until a consumer true positive exists.
- Advisor's judgment `[inference]`: "supersede the glossary half of ADR 0027", as report B proposed, is not expressible, because supersede is one-to-one; a new ADR must supersede ADR 0027 whole. `skills/living-docs/templates/context-index.md` still exists and would give vocabulary two homes.
- Advisor's judgment `[inference]`: `blocked_by` is free text today (`living-docs-core/src/commands/set/issue_keys.rs`; no `blocked_by` in `living-docs-core/src/check`), so a dangling-reference check needs a decided referent first. Kahn layers [57] answer "what can start in parallel", not "what ships together".
- Advisor's judgment `[inference]`: a `Decision:` commit trailer, ranked first by report C, is a second gate and invites a reflexive `none`; governance globs in `check` repeat Deprecated ADR 0032 and the stale-impact class ADR 0057 removed.
- Advisor's judgment `[inference]`: a `domain` type plus a hook that says "declare a record" recreates the ADR+BDR pair; the hook must point at the active issue's Decision section.
- Advisor's evidence: `usecase-beta` fails `living-docs check --mermaid-only` on 0.28.0, while `classDiagram`, `erDiagram`, `C4Component` and `architecture-beta` pass. A `use-case` view kind would ship a record type its own gate rejects `[inference]`.
- Research 0004's graph source, ADR 0031, is Deprecated with no successor (ADR 0059); its first open question is answered: distillation needs a code-derived import graph from outside the tool. Research 0004 stays as written; this note supersedes its premise as evidence `[inference]`.
- Every capture mechanism lacks outcome data (Findings, AI-assisted engineering), so any hook ships as a measured experiment with a removal condition `[inference]`.
- The HTML export needs a constitution amendment, because the constitution cut "a web front" and "public export" and a local export is neither `[inference]`.

## Open Questions

- Does a hand-kept context file with Terms and Rules attract links from issues, or do issues keep restating rules?
- What share of Stop-hook prompts ends in a real edit to an issue's Decision section, and what share is noise?
- How many dangling, self-referencing or cyclic `blocked_by` entries does the relent bundle hold today?
- Which extractor fits a Rust workspace's component view: `cargo-modules` DOT [9] lifted by a declared mapping, or crate boundaries read from `Cargo.toml`?
- How large is a single-file export of the 5012-record bench bundle, and how much does vendored `mermaid.js` add to the binary?
- Will `usecase-beta` leave beta, and does merman then validate it?

# References

[1] [Structurizr — Export](https://docs.structurizr.com/export)
[2] [Structurizr for Java — Component finder](https://docs.structurizr.com/java/component)
[3] [structurizr/java — structurizr-component](https://github.com/structurizr/java/tree/master/structurizr-component)
[4] [LikeC4 — CLI](https://likec4.dev/tooling/cli/)
[5] [LikeC4 pull request 3506 — code-to-likec4 agent skill](https://github.com/likec4/likec4/pull/3506)
[6] [borwood/cb4](https://github.com/borwood/cb4)
[7] [Archyl — Import C4 architecture via MCP](https://www.archyl.com/blog/import-c4-architecture-mcp)
[8] [Mermaid — C4 diagrams](https://mermaid.js.org/syntax/c4.html)
[9] [regexident/cargo-modules](https://github.com/regexident/cargo-modules)
[10] [dependency-cruiser — CLI reference](https://github.com/sverweij/dependency-cruiser/blob/main/doc/cli.md)
[11] [deptrac documentation](https://deptrac.github.io/deptrac/)
[12] [deptrac — Formatters](https://deptrac.github.io/deptrac/formatters/)
[13] [import-linter — Contract types](https://import-linter.readthedocs.io/en/stable/contract_types/)
[14] [ArchUnit user guide](https://www.archunit.org/userguide/html/000_Index.html)
[15] [github/stack-graphs (archived)](https://github.com/github/stack-graphs)
[16] [LSIF (archived, points to SCIP)](https://lsif.dev/)
[17] [rust-analyzer issue 23604 — SCIP output](https://github.com/rust-lang/rust-analyzer/issues/23604)
[18] [k1LoW/tbls](https://github.com/k1LoW/tbls)
[19] [Atlas — Inspect](https://atlasgo.io/inspect)
[20] [Atlas — Community Edition](https://atlasgo.io/community-edition)
[21] [petgraph — algo module](https://docs.rs/petgraph/latest/petgraph/algo/index.html)
[22] [petgraph — dag_transitive_reduction_closure](https://docs.rs/petgraph/latest/petgraph/algo/tred/fn.dag_transitive_reduction_closure.html)
[23] [Mancoridis et al. — Using automatic clustering to produce high-level system organizations (IWPC 1998)](https://www.cs.drexel.edu/~bmitchell/pubs/iwpc98.pdf)
[24] [Mancoridis et al. — Bunch (ICSM 1999)](https://www.cs.drexel.edu/~bmitchell/pubs/icsm99.pdf)
[25] [GECCO 2005 — Modularization Quality robustness study](https://www.cs.york.ac.uk/rts/docs/GECCO_2005/Conference%20proceedings/docs/p1029.pdf)
[26] [Tzerpos and Holt — ACDC (WCRE 2000)](https://www.computer.org/csdl/proceedings-article/wcre/2000/08810258/12OmNrGb2fa)
[27] [arXiv 1903.06895 — architecture recovery comparison (cites Garcia et al. ASE 2013, Lutellier et al.)](https://arxiv.org/html/1903.06895v1)
[28] [Murphy, Notkin, Sullivan — Software Reflexion Models (FSE 1995)](https://dl.acm.org/doi/10.1145/222132.222136)
[29] [PlantUML](https://plantuml.com/)
[30] [Latias94/merman](https://github.com/Latias94/merman)
[31] [1jehuang/mermaid-rs-renderer](https://github.com/1jehuang/mermaid-rs-renderer)
[32] [rust-fsm](https://docs.rs/rust-fsm/latest/rust_fsm/)
[33] [python-statemachine — Diagrams](https://python-statemachine.readthedocs.io/en/latest/diagram.html)
[34] [Trysac/u040-trace-to-sequence](https://github.com/Trysac/u040-trace-to-sequence)
[35] [handoffmap 0.2.0](https://pypi.org/project/handoffmap/0.2.0/)
[36] [Business Rules Group — Business Rules Manifesto](https://www.businessrulesgroup.org/brmanifesto.htm)
[37] [Mavin — EARS](https://alistairmavin.com/ears/)
[38] [Cucumber — Gherkin reference](https://cucumber.io/docs/gherkin/reference/)
[39] [Event Storming Journal — design-level Event Storming](https://www.eventstormingjournal.com/software%20design/how-to-explain-design-level-event-storming-to-your-mother/)
[40] [jbcodeforce — Event storming](https://jbcodeforce.github.io/eda-studies/methodology/event-storming/)
[41] [ddd-crew — Bounded Context Canvas](https://github.com/ddd-crew/bounded-context-canvas)
[42] [arc42 — Section 8, Crosscutting Concepts](https://docs.arc42.org/section-8/)
[43] [Doorstop documentation](https://doorstop.readthedocs.io/en/latest/)
[44] [StrictDoc — User guide](https://strictdoc.readthedocs.io/en/stable/stable/docs/strictdoc_01_user_guide.html)
[45] [OpenFastTrace — System requirements](https://github.com/itsallcode/openfasttrace/blob/main/doc/spec/system_requirements.md)
[46] [sphinx-needs — need directive](https://sphinx-needs.readthedocs.io/en/latest/directives/need.html)
[47] [OpenSpec — Concepts](https://github.com/Fission-AI/OpenSpec/blob/main/docs/concepts.md)
[48] [Fission-AI/OpenSpec](https://github.com/Fission-AI/OpenSpec)
[49] [Kiro — Feature specs](https://kiro.dev/docs/specs/feature-specs/)
[50] [Kiro — Specs](https://kiro.dev/docs/specs/)
[51] [GitHub Spec Kit — tasks template](https://github.com/github/spec-kit/blob/main/templates/tasks-template.md)
[52] [github/spec-kit](https://github.com/github/spec-kit)
[53] [Fowler — Ubiquitous Language](https://martinfowler.com/bliki/UbiquitousLanguage.html)
[54] [Fowler — Bounded Context](https://martinfowler.com/bliki/BoundedContext.html)
[55] [Context Mapper — Language model](https://contextmapper.org/docs/language-model/)
[56] [Vale — Vocabularies](https://docs.vale.sh/keys/vocabularies)
[57] [Topological sorting](https://en.wikipedia.org/wiki/Topological_sorting)
[58] [Shape Up — chapter 8](https://basecamp.com/shapeup/2.2-chapter-08)
[59] [Shape Up — chapter 3](https://basecamp.com/shapeup/1.2-chapter-03)
[60] [Patton — The new backlog](https://jpattonassociates.com/the-new-backlog/)
[61] [GitHub Docs — About milestones](https://docs.github.com/en/issues/using-labels-and-milestones-to-track-work/about-milestones)
[62] [GitHub Docs — Creating issue dependencies](https://docs.github.com/en/issues/tracking-your-work-with-issues/using-issues/creating-issue-dependencies)
[63] [Linear — Cycles](https://linear.app/docs/use-cycles)
[64] [ProdPad — Now/Next/Later roadmap](https://www.prodpad.com/blog/invented-now-next-later-roadmap/)
[65] [Tessl — Spec-driven framework and registry](https://tessl.io/blog/tessl-launches-spec-driven-framework-and-registry)
[66] [Tessl — Skills are software](https://tessl.io/blog/skills-are-software-and-they-need-a-lifecycle-introducing-skills-on-tessl)
[67] [Tessl — Task Evals](https://tessl.io/blog/introducing-task-evals-measure-whether-your-skills-actually-work)
[68] [bmad-code-org/BMAD-METHOD](https://github.com/bmad-code-org/BMAD-METHOD)
[69] [OpenAI — Agentic AI Foundation](https://openai.com/index/agentic-ai-foundation/)
[70] [Linux Foundation — AAIF formation](https://aaif.io/news/linux-foundation-announces-formation-of-aaif)
[71] [Claude Code — Hooks](https://code.claude.com/docs/en/hooks)
[72] [Anthropic — Effective context engineering for AI agents](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)
[73] [arXiv 2510.04618 — ACE](https://arxiv.org/abs/2510.04618)
[74] [Search Engine Journal — 97% of llms.txt files got no requests](https://www.searchenginejournal.com/97-of-llms-txt-files-got-no-requests-ahrefs-data-shows/579478/)
[75] [adr/adr-tools](https://github.com/adr/adr-tools)
[76] [thomvaill/log4brains](https://github.com/thomvaill/log4brains)
[77] [Software Engineering at Google — chapter 10, Documentation](https://abseil.io/resources/swe-book/html/ch10.html)
[78] [arXiv 2602.20478 — Codified Context](https://arxiv.org/abs/2602.20478)
[79] [Thoughtworks Technology Radar — Techniques](https://www.thoughtworks.com/radar/techniques)
[80] [Thoughtworks Technology Radar — Tools](https://www.thoughtworks.com/radar/tools)
[81] [Codex — Hooks](https://learn.chatgpt.com/docs/hooks)
[82] [Cursor — Hooks](https://cursor.com/docs/hooks)
[83] [Böckeler — Understanding spec-driven development: Kiro, spec-kit and Tessl](https://martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html)
[84] [Optivem Journal — Spec-driven development](https://journal.optivem.com/p/spec-driven-development-make-specs)
[85] [arXiv 2602.11988 — Gloaguen et al., Evaluating AGENTS.md](https://arxiv.org/abs/2602.11988)
[86] [arXiv 2607.27250 — Khatri, Do context files help](https://arxiv.org/abs/2607.27250)
[87] [arXiv 2601.20404 — Lulla et al., AGENTS.md efficiency](https://arxiv.org/abs/2601.20404)
[88] [arXiv 2605.10039 — McMillan, instruction adherence](https://arxiv.org/abs/2605.10039)
[89] [arXiv 2507.15003 — AIDev dataset](https://arxiv.org/abs/2507.15003)
[90] [Aghajani et al. — Software Documentation Issues Unveiled (ICSE 2019)](https://2019.icse-conferences.org/details/icse-2019-Technical-Papers/49/Software-Documentation-Issues-Unveiled)
[91] [Tan, Wagner, Treude — Detecting outdated code element references (EMSE 2024)](https://link.springer.com/article/10.1007/s10664-023-10397-6)
[92] [arXiv 2307.04291 — DOCER GitHub Action](https://arxiv.org/abs/2307.04291)
[93] [arXiv 2010.01625 — Panthaplackel et al., just-in-time inconsistency detection](https://arxiv.org/abs/2010.01625)
[94] [gastownhall/beads pull request 8221](https://github.com/gastownhall/beads/pull/8221)
[95] [arXiv 2606.09090 — Treude and Baltes, Context rot](https://arxiv.org/abs/2606.09090)
[96] [arXiv 2610.27381 — De Michele and Tantithamthavorn, Documentation drift in AI-generated PRs](https://arxiv.org/abs/2610.27381)
[97] [arXiv 2608.11095 — Chakrabarti, Catastrophic remembering](https://arxiv.org/abs/2608.11095)
[98] [arXiv 2511.12884 — Agent READMEs](https://arxiv.org/abs/2511.12884)
[99] [Chroma — Context Rot](https://research.trychroma.com/context-rot)
[100] [arXiv 2403.01709 — Dhar et al., LLMs generating design decisions (ICSA 2024)](https://arxiv.org/abs/2403.01709)
[101] [arXiv 2609.03721 — Karan et al., extracting design decisions from commits](https://arxiv.org/abs/2609.03721)
[102] [arXiv 2602.07609 — Su et al., LLMs detecting ADR violations](https://arxiv.org/abs/2602.07609)
[103] [arXiv 2604.04990 — Konrad et al., Architecture Without Architects](https://arxiv.org/abs/2604.04990)
[104] [Buchgeher et al. — Using ADRs in open source projects (IEEE Access 2023)](https://research.jku.at/en/publications/using-architecture-decision-records-in-open-source-projects-an-ms/)
[105] [arXiv 2604.19400 — CASCADE](https://arxiv.org/abs/2604.19400)
[106] [arXiv 2504.08725 — DocAgent](https://arxiv.org/abs/2504.08725)
[107] [Danger — Node library tutorial](https://danger.systems/js/tutorials/node-library)
[108] [Zhang et al. — Pull request templates (IST 2022)](https://www.sciencedirect.com/science/article/abs/pii/S0950584921002354)
[109] [arXiv 2607.21832 — agent PRs identified by Co-Authored-By](https://arxiv.org/abs/2607.21832)
[110] [pagefind — Rust API](https://docs.rs/pagefind/latest/pagefind/api/)
[111] [pagefind releases](https://github.com/pagefind/pagefind/releases)
[112] [MDN — CORS request not HTTP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CORS/Errors/CORSRequestNotHttp)
[113] [lucaong/minisearch](https://github.com/lucaong/minisearch)
[114] [Material for MkDocs — Changelog](https://squidfunk.github.io/mkdocs-material/changelog/)
[115] [The collapse of MkDocs](https://fpgmaas.com/blog/collapse-of-mkdocs/)
[116] [voix.md — Material for MkDocs maintenance mode](https://www.voix.md/blog/material-for-mkdocs-maintenance-mode)
[117] [Backstage issue 33990 — TechDocs beyond MkDocs](https://github.com/backstage/backstage/issues/33990)
[118] [Mermaid — Community integrations](https://mermaid.js.org/ecosystem/integrations-community.html)
[119] [distr.sh — Docs with Starlight](https://distr.sh/blog/distr-docs/)
[120] [mdBook — Changelog](https://github.com/rust-lang/mdBook/blob/main/CHANGELOG.md)
[121] [Zola — Page](https://www.getzola.org/documentation/content/page/)
[122] [Zola — Internal links](https://www.getzola.org/documentation/content/linking/)
[123] [Quartz — Features](https://quartz.jzhao.xyz/features/)
[124] [Quartz — Migrating](https://quartz.jzhao.xyz/getting-started/migrating)
[125] [Log4brains releases](https://github.com/thomvaill/log4brains/releases)
[126] [mrwilson/adr-viewer](https://github.com/mrwilson/adr-viewer)
