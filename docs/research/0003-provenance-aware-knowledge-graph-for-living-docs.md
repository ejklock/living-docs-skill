---
type: Research
title: Provenance-aware knowledge graph for living-docs
description: "Evidence for evolving living-docs into a provenance-aware doc-code knowledge graph: what transfers from Graphiti without an LLM, which storage serves it, and whether ProbLog earns its complexity."
status: Draft
timestamp: 2026-08-05T20:09:05Z
---

# 0003. Provenance-aware knowledge graph for living-docs

## Question

Can living-docs evolve into a provenance-aware knowledge graph that links records to code symbols, with Graphiti-style bi-temporality, while it keeps the determinism boundary of [ADR 0001](../adr/0001-living-docs-cli.md)? Do the components proposed in the incoming handoff — a dedicated graph database (Neo4j) and a probabilistic inference layer (ProbLog) — earn their complexity?

## Method

Falsify-mode deep research, evidence gathered on 2026-08-05. The working hypothesis: living-docs gains doc-code lineage by promoting its implicit edges (supersede, frontmatter refs, body links) to a first-class graph in the existing relational store, and by adopting Graphiti's bi-temporal edge-invalidation model without its LLM extraction layer; a ProbLog layer does not pay its cost at this scale.

Six parallel search angles ran refute-first against four named falsifiers, plus one exploration of the living-docs codebase (workspace at v0.10.1). Sources followed the priority ladder: primary papers and source code first, official specs and vendor docs second, practitioner reports third. Load-bearing claims were cross-checked across at least two independent sources where such sources exist. Roughly 60 sources were reviewed; the ones cited below carried the verdicts.

Falsifier verdicts:

- **F1 — "Graphiti's bi-temporal value is inseparable from LLM extraction": REFUTED.** The invalidation state machine (`resolve_edge_contradictions`) is pure datetime-interval logic with zero LLM calls [5]; `add_triplet` accepts pre-structured input and skips extraction [6]; timestamp extraction is skipped when dates are pre-set [5]. LLMs serve only as the parser for unstructured prose.
- **F2 — "Relational storage cannot serve the traversals": REFUTED at this scale.** Documented breakdown thresholds sit two orders of magnitude above the living-docs workload [22][29]. Caveat: keep traversals depth-bounded; unbounded path enumeration over dense graphs degrades combinatorially [22].
- **F3 — "Deterministic doc-code anchors rot faster than they help": UNRESOLVED, leaning refuted.** No tier-1 study directly compares checked anchors against ML recovery under refactoring; the benchmark does not exist [21]. Indirect evidence favors checked anchors: the decay driver is refactoring, which structural detection handles [17]; checked anchors turn silent rot into loud breakage, the failure mode developers actually miss [15]; ML recovery is not a viable fallback [14].
- **F4 — "A probabilistic layer is load-bearing": REFUTED.** No comparable provenance system uses probabilistic inference [33][35][1]; production systems that need graded weighting use deterministic exponential decay [36]; ProbLog is Python-only, Beta, with #P-complete exact inference [30][31]; Scallop has a Rust core but is a v0.2 research prototype [32].

Known limits of this run: two academic searches (Phelps & Wilensky robust locations; LHDiff) were blocked by search-safety filters, so the annotation-anchoring literature thread is under-evidenced; all F1 evidence originates from Zep (paper plus repo), mitigated by reading the source code directly rather than trusting prose claims.

## Findings

| Finding | Evidence | Confidence |
|---|---|---|
| Graphiti's bi-temporal engine is deterministic: four timestamps per edge (`created_at`/`expired_at` on the system timeline, `valid_at`/`invalid_at` on the event timeline) and interval-logic invalidation; LLMs only parse unstructured input | [1][2][4][5][6] | high |
| At living-docs scale (~10^3 nodes, ~10^4 edges), SQLite recursive CTEs traverse with two orders of magnitude of headroom; graph-extension layers (Apache AGE) benchmark slower than hand-written CTEs | [22][23][29] | high |
| Every embedded graph engine is a dependency risk as of late 2025: Kuzu archived after the Apple acquisition, CozoDB abandoned, IndraDB dormant, Neo4j JVM-only, SurrealDB BSL-licensed; Graphiti itself deprecated its only embedded backend | [24][25][26][28][3] | high |
| Bi-temporal modeling reduces to the SQL:2011 four-column pattern (`valid_from`/`valid_to`, `system_from`/`system_to`) with AS OF queries, implementable in plain SQLite — XTDB is the prior art | [27] | high |
| Plain-text code references in docs rot silently in most projects; the decay driver is rename/move/deprecation refactoring; structural refactoring-aware link evolution beats re-running IR recovery; automated recovery is "not sufficiently good" for high-assurance use | [15][16][17][14] | high |
| Maintained, trustworthy trace links pay: developers were 24% faster and 50% more correct in the one strong controlled experiment (n=71) | [13][43] | medium |
| Position-independent symbol identity is proven practice: SCIP symbols are FQN strings with no positional component; stack-graphs bind names deterministically per file from tree-sitter; rename/move resilience is heuristic in every shipped system — no tool maintains a declared cross-commit identity | [7][8][9][10][12] | high |
| No comparable provenance system uses probabilistic inference: W3C PROV-DM models validity as deterministic events with no confidence attribute; OpenLineage is deterministic run events; Graphiti uses recency-wins invalidation; graded weighting in production is exponential decay | [33][34][35][1][36] | high |
| Market gap: no tool combines decision-record lifecycle, AST-identity doc-code anchors, and deterministic staleness verdicts — Swimm is snippet-level and heuristic, ADR tooling has zero code links, Glean/potpie treat docs as generation targets or RAG context | [37][38][39][40] | medium |
| In OSS, missing trace links (47% of release artifacts) dominate broken links (12%): non-creation, not decay, is the larger failure mode — anchor-creation friction must be near zero | [20] | medium |
| The graph substrate already exists in this repo: db-store has a `relations` table (today only `kind="supersede"`), records carry `status`/`revision`/`deleted_at`, FTS5 is in place; body links are validated by `check` but never persisted as edges | living-docs v0.10.1 source: `db-store/src/migration.rs`, `db-store/src/sync.rs:313-347`, `living-docs-core/src/check/links.rs` | high |

Contradicting evidence recorded: Swimm's commercial success rests on heuristic anchor re-binding [37], which argues that heuristics have value — the resolution is placement, not rejection: heuristic re-binding belongs to the authoring model outside the tool, while the tool fails loudly. Human vetting of machine-generated links degrades high-quality matrices [18], so "add a human in the loop" is not a free correctness upgrade.

## Implications

This evidence supports: promoting the existing `relations` table to a typed, bi-temporal edge set (SQL:2011 four-column pattern) inside the current SQLite/Postgres db-store; anchoring records to code via SCIP-style position-independent symbol identifiers; treating renames as declared lineage events recorded at refactor time, not guessed; making the doc-gate emit deterministic staleness verdicts (loud breakage over silent rot); and expressing any graded "confidence" as status enums plus optional recency decay computed at query time.

This evidence rules out: adopting Neo4j or any embedded graph engine (dependency risk without payoff at this scale); a ProbLog/probabilistic-logic inference layer (no peer-system precedent, no Rust path, #P-complete); and heuristic anchor re-binding inside the tool (belongs to the authoring model per the determinism boundary).

This repo already holds a compatible design: [issue 0024](../issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) specifies commit trailers, `covers:`-glob drift detection, and executable acceptance. The evidence here maps onto it directly — trailers are declared lineage events (Graphiti's episode pattern), `drift` is the loud-breakage staleness verdict, and the additions this research motivates are symbol-level (FQN) anchors as a finer grain than path globs, plus the bi-temporal edge model over the existing `relations` table.

The decisions themselves belong to ADRs that this note feeds; it makes none of them.

## Open Questions

- No benchmark or dataset exists for ADR-to-code traceability [21]; early telemetry from dogfooding is the only evidence substitute for the staleness-verdict design.
- The half-life of FQN anchors under real refactoring is unmeasured; instrument it before tuning any decay weights.
- Whether graded confidence scoring is wanted at all, versus plain status + reachability — no surveyed system needed it.
- How doc-code edges surface in the web front (read-only projection vs. authoring) is untouched by this research.
- The academic robust-anchoring thread (Phelps & Wilensky; LHDiff) remains unread due to blocked searches.

# References

[1] [RASMUSSEN, P. et al. Zep: A Temporal Knowledge Graph Architecture for Agent Memory. arXiv, 2025](https://arxiv.org/abs/2501.13956). Available at: https://arxiv.org/abs/2501.13956. Accessed on: 2026-08-05.
[2] [Zep paper, HTML full text v1](https://arxiv.org/html/2501.13956v1). Available at: https://arxiv.org/html/2501.13956v1. Accessed on: 2026-08-05.
[3] [GETZEP. graphiti README. GitHub, 2026](https://github.com/getzep/graphiti). Available at: https://github.com/getzep/graphiti. Accessed on: 2026-08-05.
[4] [GETZEP. graphiti_core/edges.py](https://github.com/getzep/graphiti/blob/main/graphiti_core/edges.py). Available at: https://github.com/getzep/graphiti/blob/main/graphiti_core/edges.py. Accessed on: 2026-08-05.
[5] [GETZEP. graphiti_core/utils/maintenance/edge_operations.py](https://github.com/getzep/graphiti/blob/main/graphiti_core/utils/maintenance/edge_operations.py). Available at: https://github.com/getzep/graphiti/blob/main/graphiti_core/utils/maintenance/edge_operations.py. Accessed on: 2026-08-05.
[6] [GETZEP. graphiti_core/graphiti.py (add_triplet)](https://github.com/getzep/graphiti/blob/main/graphiti_core/graphiti.py). Available at: https://github.com/getzep/graphiti/blob/main/graphiti_core/graphiti.py. Accessed on: 2026-08-05.
[7] [SOURCEGRAPH. SCIP protobuf schema (scip.proto)](https://github.com/sourcegraph/scip/blob/main/scip.proto). Available at: https://github.com/sourcegraph/scip/blob/main/scip.proto. Accessed on: 2026-08-05.
[8] [SOURCEGRAPH. Announcing SCIP. 2022](https://sourcegraph.com/blog/announcing-scip). Available at: https://sourcegraph.com/blog/announcing-scip. Accessed on: 2026-08-05.
[9] [GITHUB. Introducing stack graphs. 2021](https://github.blog/open-source/introducing-stack-graphs/). Available at: https://github.blog/open-source/introducing-stack-graphs/. Accessed on: 2026-08-05.
[10] [GIT. git-blame documentation (-M/-C move detection)](https://git-scm.com/docs/git-blame). Available at: https://git-scm.com/docs/git-blame. Accessed on: 2026-08-05.
[11] [GERRIT. REST API — ported_comments](https://gerrit-review.googlesource.com/Documentation/rest-api-changes.html). Available at: https://gerrit-review.googlesource.com/Documentation/rest-api-changes.html. Accessed on: 2026-08-05.
[12] [GRUND, F.; DUALA-EKOKO, E. et al. CodeShovel: Unearthing Method Histories. ICSE, 2021](https://github.com/ataraxie/codeshovel). Available at: https://github.com/ataraxie/codeshovel. Accessed on: 2026-08-05.
[13] [MÄDER, P.; EGYED, A. Do developers benefit from requirements traceability? Empirical Software Engineering, 2015](https://link.springer.com/article/10.1007/s10664-014-9314-z). Available at: https://link.springer.com/article/10.1007/s10664-014-9314-z. Accessed on: 2026-08-05.
[14] [GUO, J.; STEGHÖFER, J.-P.; VOGELSANG, A.; CLELAND-HUANG, J. Natural Language Processing for Requirements Traceability. arXiv, 2024](https://arxiv.org/abs/2405.10845). Available at: https://arxiv.org/abs/2405.10845. Accessed on: 2026-08-05.
[15] [TAN, W.; WAGNER, S.; TREUDE, C. Detecting Outdated Code Element References in Software Repository Documentation. arXiv, 2022](https://arxiv.org/abs/2212.01479). Available at: https://arxiv.org/abs/2212.01479. Accessed on: 2026-08-05.
[16] [WEN, F. et al. A Large-Scale Empirical Study on Code-Comment Inconsistencies. ICPC, 2019](https://dl.acm.org/doi/abs/10.1109/ICPC.2019.00019). Available at: https://dl.acm.org/doi/abs/10.1109/ICPC.2019.00019. Accessed on: 2026-08-05.
[17] [RAHIMI, M.; CLELAND-HUANG, J. Evolving software trace links between requirements and source code. Empirical Software Engineering, 2018](https://link.springer.com/article/10.1007/s10664-017-9561-x). Available at: https://link.springer.com/article/10.1007/s10664-017-9561-x. Accessed on: 2026-08-05.
[18] [NIU, N. et al. Gray Links in the Use of Requirements Traceability. FSE, 2016](https://homepages.uc.edu/~niunn/papers/FSE16.pdf). Available at: https://homepages.uc.edu/~niunn/papers/FSE16.pdf. Accessed on: 2026-08-05.
[19] [BUCHGEHER, G. et al. Using Architecture Decision Records in Open Source Projects. IEEE Access, 2023](https://ieeexplore.ieee.org/document/10155430/). Available at: https://ieeexplore.ieee.org/document/10155430/. Accessed on: 2026-08-05.
[20] [Establishing Traceability between Release Notes & Software Artifacts: Practitioners' Perspectives. arXiv, 2025](https://arxiv.org/abs/2511.18187). Available at: https://arxiv.org/abs/2511.18187. Accessed on: 2026-08-05.
[21] [KIT SDQ. Trace Link Recovery for Architecture Decision Records](https://sdq.kastel.kit.edu/wiki/Trace_Link_Recovery_for_Architecture_Decision_Records_(ADRs)). Available at: https://sdq.kastel.kit.edu/wiki/Trace_Link_Recovery_for_Architecture_Decision_Records_(ADRs). Accessed on: 2026-08-05.
[22] [SQLITE. The WITH Clause — Queries Against A Graph](https://sqlite.org/lang_with.html). Available at: https://sqlite.org/lang_with.html. Accessed on: 2026-08-05.
[23] [PAPATHANASIOU, D. simple-graph: a graph database in SQLite](https://github.com/dpapathanasiou/simple-graph). Available at: https://github.com/dpapathanasiou/simple-graph. Accessed on: 2026-08-05.
[24] [THE REGISTER. KuzuDB graph database abandoned, community mulls options. 2025](https://www.theregister.com/software/2025/10/14/kuzudb-graph-database-abandoned-community-mulls-options/). Available at: https://www.theregister.com/software/2025/10/14/kuzudb-graph-database-abandoned-community-mulls-options/. Accessed on: 2026-08-05.
[25] [DBDB.IO. CozoDB entry (abandoned since 2024)](https://dbdb.io/db/cozodb). Available at: https://dbdb.io/db/cozodb. Accessed on: 2026-08-05.
[26] [OXIGRAPH. SPARQL graph database in Rust](https://github.com/oxigraph/oxigraph). Available at: https://github.com/oxigraph/oxigraph. Accessed on: 2026-08-05.
[27] [XTDB. Time in XTDB — SQL:2011 bitemporal model](https://docs.xtdb.com/about/time-in-xtdb.html). Available at: https://docs.xtdb.com/about/time-in-xtdb.html. Accessed on: 2026-08-05.
[28] [NEO4J. Using Neo4j embedded in Java applications. v2026.06](https://neo4j.com/docs/java-reference/current/java-embedded/). Available at: https://neo4j.com/docs/java-reference/current/java-embedded/. Accessed on: 2026-08-05.
[29] [EXOBENCH. How Fast Are Postgres 19 Graph Queries? Part 1. 2026](https://exobench.ai/blog/pg19-graph-queries-part-1). Available at: https://exobench.ai/blog/pg19-graph-queries-part-1. Accessed on: 2026-08-05.
[30] [PYPI. problog 2.2.10 (Beta). 2026](https://pypi.org/project/problog/). Available at: https://pypi.org/project/problog/. Accessed on: 2026-08-05.
[31] [KU LEUVEN DTAI. ProbLog — How it works](https://dtai.cs.kuleuven.be/problog/). Available at: https://dtai.cs.kuleuven.be/problog/. Accessed on: 2026-08-05.
[32] [SCALLOP-LANG. Scallop (Rust core, provenance semirings)](https://github.com/scallop-lang/scallop). Available at: https://github.com/scallop-lang/scallop. Accessed on: 2026-08-05.
[33] [W3C. PROV-DM: The PROV Data Model. Recommendation, 2013](https://www.w3.org/TR/prov-dm/). Available at: https://www.w3.org/TR/prov-dm/. Accessed on: 2026-08-05.
[34] [W3C. PROV Overview. 2013](https://www.w3.org/TR/prov-overview/). Available at: https://www.w3.org/TR/prov-overview/. Accessed on: 2026-08-05.
[35] [OPENLINEAGE. Object Model specification](https://openlineage.io/docs/spec/object-model/). Available at: https://openlineage.io/docs/spec/object-model/. Accessed on: 2026-08-05.
[36] [MILVUS. Exponential Decay reranker documentation](https://milvus.io/docs/exponential-decay.md). Available at: https://milvus.io/docs/exponential-decay.md. Accessed on: 2026-08-05.
[37] [SWIMM. How does Swimm's Auto-sync feature work? 2021. [COI: vendor]](https://swimm.io/blog/how-does-swimm-s-auto-sync-feature-work). Available at: https://swimm.io/blog/how-does-swimm-s-auto-sync-feature-work. Accessed on: 2026-08-05.
[38] [VAILL, T. log4brains — ADR knowledge base](https://github.com/thomvaill/log4brains). Available at: https://github.com/thomvaill/log4brains. Accessed on: 2026-08-05.
[39] [META. Indexing code at scale with Glean. 2024](https://engineering.fb.com/2024/12/19/developer-tools/glean-open-source-code-indexing/). Available at: https://engineering.fb.com/2024/12/19/developer-tools/glean-open-source-code-indexing/. Accessed on: 2026-08-05.
[40] [POTPIE-AI. potpie — Context Graph. [COI: vendor]](https://github.com/potpie-ai/potpie). Available at: https://github.com/potpie-ai/potpie. Accessed on: 2026-08-05.
[41] [BARBERO, M. Understanding Software Provenance Attestation: SLSA and in-toto. 2023](https://mikael.barbero.tech/blog/post/2023-12-28-slsa-and-in-toto/). Available at: https://mikael.barbero.tech/blog/post/2023-12-28-slsa-and-in-toto/. Accessed on: 2026-08-05.
[42] [GITHUB. stack-graphs repository](https://github.com/github/stack-graphs). Available at: https://github.com/github/stack-graphs. Accessed on: 2026-08-05.
[43] [The Impact of Traceability on Software Maintenance and Evolution: A Mapping Study. arXiv, 2021](https://arxiv.org/abs/2108.02133). Available at: https://arxiv.org/abs/2108.02133. Accessed on: 2026-08-05.
