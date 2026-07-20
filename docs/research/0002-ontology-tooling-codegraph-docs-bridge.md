---
type: Research
title: Ontology tooling and the code-to-docs bridge — Rust-native options for a living-docs knowledge layer
description: A source-cited survey of open-source ontology tooling evaluated as the bridge between codegraph's symbol graph and the living-docs semantic index. Finding — an embedded, Rust-native RDF store (Oxigraph) with a SKOS concept scheme is the least-cost substrate; SKOS is sufficient until logical reasoning is required; the wiki prior art is Dendron+Cargo, not Semantic MediaWiki/Wikibase; and the bridge layer should be built only after an eval corpus shows FTS+embeddings failing.
status: Accepted
supersedes:
superseded_by:
tags: [research, ontology, knowledge-graph, skos, rdf, oxigraph, codegraph, semantic-index, wiki, rust]
timestamp: 2026-07-20T00:00:00Z
---

# Ontology Tooling and the Code-to-Docs Bridge — Rust-Native Options for a Living-Docs Knowledge Layer

Compiled: 2026-07-20
Scope: open-source ontology substrates and formats evaluated for one job — bridging codegraph's code-symbol graph and the living-docs semantic index ("from code up to the human, and back"), under this repo's locked constraints (one language / one build; determinism boundary; "every doc lands in exactly one place").
Epistemic mode: `falsify` (a working hypothesis existed). Depth: `standard`.

---

## Method

Six independent retrieval angles ran in parallel, chosen **refute-first** so each falsifier of the working hypothesis got a dedicated angle. Every angle was instructed to prefer tier-1 sources (W3C specs, arХiv/peer-reviewed papers, primary project repos and their crates.io/GitHub API metadata) and to report blocked fetches rather than omit them.

**Working hypothesis.** A lightweight, Rust-native ontology layer — a SKOS controlled vocabulary held in an embedded RDF store (Oxigraph), with sophia as the toolkit — can bridge the codegraph symbol graph and the living-docs semantic index without introducing a non-Rust service, and Semantic MediaWiki/Wikibase are the best prior art for the wiki+ontology surface.

**Falsifiers and their verdicts** (graded from the evidence below):

| # | Falsifier | Verdict |
|---|---|---|
| F1 | The Rust RDF ecosystem (Oxigraph/sophia/horned-owl) is immature or unmaintained, forcing a non-Rust service. | **REFUTED** |
| F2 | SKOS is too weak for the code↔human bridge; OWL inference is actually required. | **REFUTED** (conditional) |
| F3 | A code-ontology standard (SCIP/Kythe/CodeOntology) or a property-graph service beats RDF-in-Rust. | **REFUTED** for the bridge; wins only for the code-symbol layer |
| F4 | The Semantic MediaWiki/Wikibase model does not fit a docs-per-project wiki; other prior art fits better. | **CONFIRMED** — hypothesis updated |

**Decision at stake.** Which ontology substrate + serialization the planned "Living Docs Atlas" wiki adopts, and whether it stays Rust-native — feeding the future ADRs on repo structure (monorepo vs split) and the wiki PRD.

**Blocked / weakened fetches (confidence downgraded accordingly, never silently dropped):** the ER-2024 *Requirements2Code* PDF returned FlateDecode binary — its claim rests on a search summary, marked `[unverified — single source]`; the *Code Digital Twin* (arХiv 2503.07967) "code symbol graph vs domain knowledge graph" phrasing is a summarizer paraphrase — only the "physical and conceptual layers … hybrid knowledge representations" wording is verbatim; the W3C working draft *Using OWL and SKOS* extracted only generic pattern language, so the Concept-vs-Class point rests on the SKOS Reference itself; GitHub contributor-graph counts for sophia/horned-owl did not render (maintenance judged from commits/issues/stars instead); no tier-1 source benchmarks JVM-vs-Rust integration overhead, so that comparison is qualitative/architectural, marked `[inference]`.

---

## Findings

### F1 — The Rust RDF ecosystem is production-viable as an embedded library; Oxigraph is the strongest [confidence: high]

**Oxigraph** is an actively developed, embeddable, in-process RDF store implementing the SPARQL 1.1 standard, with a RocksDB-backed on-disk store plus an in-memory mode [4]. As of this survey its latest release is 0.5.9 (2026-06-18) on a near-monthly cadence, with commits landing within a day of the survey date, a healthy open:closed issue ratio (~112:247), and named commercial users [4][5]. Its one explicit caveat is a performance note in its own README — "SPARQL query evaluation has not been optimized yet" — a performance caveat, not a "not for production" warning [4]. **horned-owl** (OWL 2 DL modeling/parsing, academically backed, peer-reviewed 20–40× speedups over the Java OWL API) released a 2.1.0 major bump three days before the survey and is clearly active — but it is a modeling library, not a store or reasoner (reasoning lives in the companion whelk-rs) [7][8]. **sophia** is standards-current (RDF 1.2, JSON-LD 1.1) but its SPARQL is explicitly "partial" with no Update and no in-tree persistent store, making it a toolkit layer rather than a database [6].

Two real qualifications temper F1's refutation. First, **SKOS has no actively-maintained dedicated Rust crate**: `rdftk_skos` has been dormant since 2021-06-14 and self-describes as limited [9]; SKOS in Rust therefore means building on a general RDF library (Oxigraph/sophia) with `rdf_vocabularies`, or the newer `oxirs-rule` validation module — not adopting a turnkey SKOS crate. Second, all three core libraries lean on a **single principal maintainer** — the ecosystem's real bus-factor risk. Neither qualification points back toward a non-Rust service; both are absorbed by treating SKOS as a vocabulary you assert over Oxigraph rather than a library you import.

### F2 — SKOS is sufficient for the glossary + typed cross-scheme links; OWL is required only for logical reasoning [confidence: high]

Every capability the bridge names maps to a native SKOS construct: `skos:Concept` with `prefLabel`/`altLabel`/`hiddenLabel`/`notation` for glossary terms; `skos:ConceptScheme` to separate the docs vocabulary from the code-symbol vocabulary; and the mapping relations `exactMatch`/`closeMatch`/`broadMatch`/`narrowMatch`/`relatedMatch` for typed cross-scheme links between them [1]. This is exactly the pattern that LCSH, Getty AAT, EuroVoc (≈202k prefLabels, ≈3.6M relations) and AGROVOC (≈5.9M relations) run in production at scale [1][35]. The W3C is explicit that this is a **deliberate design line, not a defect**: "SKOS is not a formal knowledge representation language" [1], and `skos:broader` is deliberately non-transitive with an opt-in `skos:broaderTransitive` closure [1][2]. The single specific requirement that forces a move up to OWL is a need to **compute new facts by logical entailment** — enforcing class axioms (disjointness, domain/range or cardinality restrictions), running a reasoner to detect contradictions or auto-classify a symbol under a concept, or treating each glossary term as an `owl:Class` so instances inherit through it [33]. The documented real-world escalations (EMMeT, Digital Europa Thesaurus) were all triggered by exactly that reasoning demand [33]; the one non-reasoning escalation (LCSH → MADS/RDF) was triggered by needing to model a heading's internal component structure, not by reasoning [34].

### F3a — No off-the-shelf standard bridges code symbols to human doc concepts; the code-symbol layer alone has mature standards [confidence: high]

The industrial code-intelligence standards — LSIF (Microsoft), SCIP (Sourcegraph), and Kythe (Google) — are purpose-built graphs (Protobuf / labeled-property, not RDF/OWL) that model definitions, references, hovers, and types and **stop at the code boundary**; none carries a documentation-concept or SKOS layer, and SCIP explicitly disclaims being a queryable/semantic store [10][11][12]. Code Property Graphs (Joern) are a security/dataflow property graph, also non-RDF and doc-agnostic [15]. The one project that genuinely does code→RDF/OWL *and* a documentation-to-concept bridge, **CodeOntology** (OWL 2 + a parser serializing Java to RDF triples, linking doc comments to DBpedia via TagMe), is precisely the design a hand-rolled bridge would emulate — but it is Java-only and unmaintained since ~2021–2022 [13][14]. The SEON ontologies are SE-domain *reference* ontologies, not per-repo symbol indexers. Consequently the current research frontier treats the code-symbol↔NL-concept link as something you **build or learn** (heterogeneous GNNs, embeddings, or LLM trace-link recovery), and states outright that the domain-concept graph must be supplied [16][17]. **Net:** codegraph is already the right kind of artifact for the code-symbol side (a SCIP/Kythe-shaped graph); nothing standard covers the bridge, so a hand-rolled SKOS/annotation map is not made redundant.

### F3b — Embedded RDF (Oxigraph) or extending SQLite beats a property-graph service for a one-language, in-process codebase [confidence: high]

The only credible in-process, same-language options are Oxigraph (Rust RDF/SPARQL) and extending the existing SQLite graph — both single-binary, zero new runtime [4]. The embeddable property graph that could have matched them, **KuzuDB, was archived on 2025-10-10** (read-only, on-disk format never stabilized), which disqualifies it as a foundation to build on now [27]. Reaching for **Neo4j + neosemantics (n10s)** buys mature Cypher and a lossless RDF round-trip, but imposes a JVM plus a separate server, and its ontology loader is lossy — it processes `rdfs:domain`/`range` and `owl:Restriction` and "all other elements will be ignored by this loader" [24][25]. A neutral peer-reviewed benchmark shows paradigm-bridging (SPARQL-on-LPG via plugin) running 2.4–27× slower than a native triple store *before* any cross-process hop [26]. Because a "documentation ontology" with SKOS/OWL semantics is RDF's home turf (standards, W3C vocabularies), a non-Rust graph service is warranted only if the project specifically needs heavy Cypher analytics or Neo4j's operational tooling — neither of which an ontology-linking bridge implies [`inference`].

### F4 — Semantic MediaWiki/Wikibase are a poor fit for a docs-per-project wiki; Dendron + Cargo is the better prior-art template [confidence: high — hypothesis updated]

This is where the working hypothesis was **wrong**. Wikibase's unit is an atomic, individually-sourced *statement* (snak + qualifiers + references + rank), not a document; it "does not allow you to visualize or query the data stored in your wiki" on its own, is heavy to self-host, and its access control is all-or-nothing [20][21]. Semantic MediaWiki fits better (typed `[[Property::Value]]` triples, OWL/RDF export, cross-vocabulary mapping) and scales to millions of rows — its real cost is query/template-formatting performance, not data volume [19] — but its own community built **Cargo** precisely because SMW was judged too heavyweight and wrongly-grained, storing template-anchored structured data in plain SQL tables with built-in FTS and ~30–50% faster queries [22]. Crucially, RDF's many-to-many "a concept is reachable from many places" model is the **opposite** of this repo's "every doc lands in exactly one place" invariant. **Dendron** matches that invariant almost exactly: hierarchy is the primary primitive, every doc has one canonical home ("one source of truth where a note can be filed"), schemas give an optional lightweight type system, and refactor/rename commands keep links from rotting [23]. The strongest structural template for the wiki is therefore a **hybrid of Dendron (organization/authoring) + Cargo (structured, queryable layer)** — with the SKOS/RDF ontology as an *export/projection* for interoperability, not as the primary store.

### F5 — The demonstrated value of the ontology/graph bridge is conditional and narrow — measure first [confidence: high]

An ontology/graph layer earns its keep, with evidence, for three things: **entity disambiguation** (explicit edges resolve a term where embeddings blur it — LinkedIn's "uber" mismap; GraphGen4Code's 79% code→doc linking accuracy) [18][16]; **multi-hop / relational and traceability queries** ("what requirement does this class satisfy", "what breaks if I rename this concept") that FTS/vector search degrade on [16][36]; and **stable, human-readable cross-artifact identity** (SCIP string IDs and SKOS mapping relations give durable anchors that per-model embeddings cannot) [10][1]. For plain "find docs about X" retrieval, the graph adds latency (graph traversal ~200–300 ms vs sub-50 ms vector ANN), schema-maintenance cost, and complexity for no measurable gain; every credible source says measure your query distribution first and expect a hybrid (vector-seeded, graph-enriched) system, not graph-only [31]. This maps directly onto this repo's own `evidence-gate` discipline: build the bridge layer only after an eval corpus shows FTS+embeddings actually failing on the relational/disambiguation queries that matter.

### F6 — The minimal viable bridge shape, and a DDD constraint on it [confidence: medium-high]

In practice the bridge is: a code-symbol graph with human-readable stable IDs (SCIP-style — codegraph already approximates this) on one side; a lightweight human concept scheme on the other; and, between them, typed links expressed with **existing standards** rather than a bespoke schema — `skos:exactMatch`/`closeMatch`/`relatedMatch` for concept↔concept alignment, and the **W3C Web Annotation model** (Body = concept, Target = a code segment/symbol) for concept↔code-location, both serialized as **JSON-LD** so the bridge is itself a queryable graph [1][3]. A load-bearing constraint from Domain-Driven Design: the human vocabulary must be **bounded-context-scoped** — the same term legitimately means different things in different contexts, and DDD explicitly rejects a single global unified model [29][30]. Any concept↔symbol mapping must therefore be context-qualified, not global — which aligns naturally with a per-project docs structure.

---

## Contradictions

- **Pro-property-graph vs pro-RDF framing.** Vendor-adjacent sources (Neo4j) argue the property graph is the pragmatic default because relationships are first-class and edge properties are native, and that one should "layer in RDF's organizing principles only when needed" [pro-LPG, treat as `[COI: Neo4j]`]. The neutral peer-reviewed benchmark and the standards case cut the other way for *this* use: an ontology with SKOS/OWL semantics is RDF-shaped, and bridging paradigms inside a server already costs 2.4–27× [26]. The contradiction resolves on the specific workload — ontology-linking favors RDF; heavy traversal analytics favor LPG.
- **Graph adds value vs graph adds only overhead.** GraphRAG advocates and skeptics disagree; the honest synthesis (adopted above) is that the graph wins only on a specific query distribution (multi-hop, disambiguation, traceability) and is pure overhead for topical retrieval [31].

---

## Analysis

The evidence converges on a coherent picture that survives the refute-first pass. The Rust-native path is real: Oxigraph is a production-viable embedded RDF/SPARQL store that keeps the "one language, one build" locked decision intact (F1), and SKOS — asserted over that store — is the purpose-built, industry-proven vocabulary for a glossary with typed cross-scheme links, needing no OWL until a genuine reasoning requirement appears (F2). No existing standard bridges code symbols to human concepts, so codegraph (the code-symbol side) plus a hand-authored SKOS scheme (the human side) plus W3C standard link types (SKOS mapping relations + Web Annotation) is not reinventing a wheel — it is assembling the wheels that exist (F3a, F6). A non-Rust graph service is not justified by an ontology-linking workload (F3b).

The one place the hypothesis broke is instructive: the wiki's prior art is **not** Semantic MediaWiki/Wikibase (many-to-many, statement-centric, heavy) but the **Dendron+Cargo** shape (hierarchy-primary "one place", template-anchored SQL tables), which is a near-exact match for this repo's existing semantic-index invariant (F4). This reframes the ontology from "the store" to "an export/projection at the boundary" — which sits comfortably inside the determinism boundary: humans author docs and the glossary; the tool deterministically *projects* records and typed links into RDF/JSON-LD, never authoring rationale or inferring meaning.

Finally, the whole ontology/graph layer is subject to this repo's own evidence-gate: its demonstrated value is narrow (disambiguation, multi-hop/traceability, stable identity), so it should be built only after a measured gap shows FTS+embeddings failing (F5). The "from code up to the human, and back" north star is real and has a named research lineage (Code Digital Twin, GraphGen4Code) — but it is a later slice, gated by evidence, not a foundational rebuild.

---

## Recommendations

Each recommendation is caveated by the confidence of the evidence under it.

1. **Stay Rust-native; do not introduce a JVM/Neo4j service for the ontology.** [high] Adopt embedded **Oxigraph** as the RDF/SPARQL substrate *if and when* an ontology layer is built. Cheaper first step: extend the existing SQLite graph with a typed-links table shaped by SKOS, and *project* to RDF/JSON-LD at the boundary — deferring a second store until the query distribution justifies it.
2. **Model the human layer as a SKOS concept scheme, not OWL.** [high] Use `skos:Concept` + `prefLabel`/`altLabel`/`notation` for the glossary, a separate `ConceptScheme` per side (docs vocabulary vs code-symbol vocabulary), and `exactMatch`/`closeMatch`/`relatedMatch` for the typed cross-scheme links. Reserve OWL for a concrete, later reasoning requirement (consistency checking / auto-classification) — and record that trigger in an ADR when it arrives.
3. **Express concept↔code-location links with the W3C Web Annotation model, serialized as JSON-LD.** [medium-high] Body = concept, Target = code segment/symbol. This keeps the bridge a standard, queryable graph and reuses codegraph's stable symbol IDs as anchors.
4. **Scope the human vocabulary per project/bounded-context, never as one global ontology.** [high] This aligns with DDD and with the per-project docs structure.
5. **Gate the bridge layer behind an eval corpus (evidence-gate).** [high] Build a 50–100 query set from real navigation/QA needs; only build graph traversal/reasoning when FTS+embeddings measurably fail on relational/disambiguation/traceability queries. Until then, the semantic index (FTS5) already in flight is the right primitive.
6. **Emulate Dendron (organization) + Cargo (structured layer) for the wiki, not SMW/Wikibase.** [high] Hierarchy-primary "one doc, one place", optional schemas, refactor-safe links, template-anchored queryable tables — the ontology is an export, not the store.
7. **Sequencing.** [inference] This note should feed three separate decisions, in order: (a) the **wiki PRD** ("Living Docs Atlas") which sets the north star; (b) an **ADR on the ontology substrate** (Oxigraph-embedded vs SQLite-projection) written only when the evidence-gate opens; (c) the **repo-structure ADR** (monorepo vs splitting the web front), where the wiki's independent deploy cadence is the trigger to weigh. The ontology is deliberately the *last* of these to reach code.

# References

<!-- Full entries per skills/living-docs/rules/citation-conventions.md. Access date 2026-07-20. -->

[1] W3C. **SKOS Simple Knowledge Organization System Reference** (W3C Recommendation). 2009. Available at: https://www.w3.org/TR/skos-reference/. Accessed on: 2026-07-20.

[2] W3C. **SKOS Simple Knowledge Organization System Primer**. 2009. Available at: https://www.w3.org/TR/skos-primer/. Accessed on: 2026-07-20.

[3] W3C. **Web Annotation Data Model** (W3C Recommendation). 2017. Available at: https://www.w3.org/TR/annotation-model/. Accessed on: 2026-07-20.

[4] OXIGRAPH PROJECT. **Oxigraph — a graph database implementing the SPARQL standard** (repository). 2026. Available at: https://github.com/oxigraph/oxigraph. Accessed on: 2026-07-20.

[5] CRATES.IO. **oxigraph crate metadata** (0.5.9, 2026-06-18). Available at: https://crates.io/api/v1/crates/oxigraph. Accessed on: 2026-07-20.

[6] CHAMPIN, Pierre-Antoine. **sophia_rs — a Rust toolkit for RDF and Linked Data** (repository). 2026. Available at: https://github.com/pchampin/sophia_rs. Accessed on: 2026-07-20.

[7] LORD, Phillip. **horned-owl — a library for OWL ontologies in Rust** (repository). 2026. Available at: https://github.com/phillord/horned-owl. Accessed on: 2026-07-20.

[8] CRATES.IO. **horned-owl crate metadata** (2.1.0, 2026-07-17). Available at: https://crates.io/api/v1/crates/horned-owl. Accessed on: 2026-07-20.

[9] CRATES.IO. **rdftk_skos crate metadata** (0.2.0, 2021-06-14). Available at: https://crates.io/api/v1/crates/rdftk_skos. Accessed on: 2026-07-20.

[10] SOURCEGRAPH. **SCIP — a better code indexing format than LSIF**. Available at: https://sourcegraph.com/blog/announcing-scip. Accessed on: 2026-07-20.

[11] MICROSOFT. **Language Server Index Format (LSIF) Specification 0.4.0**. Available at: https://microsoft.github.io/language-server-protocol/specifications/lsif/0.4.0/specification/. Accessed on: 2026-07-20.

[12] KYTHE PROJECT. **Kythe Storage Model**. Available at: https://kythe.io/docs/kythe-storage.html. Accessed on: 2026-07-20.

[13] ATZORI, Mauro et al. **CodeOntology: RDF-ization of Source Code** (ISWC 2017). Available at: https://link.springer.com/chapter/10.1007/978-3-319-68204-4_2. Accessed on: 2026-07-20.

[14] CODEONTOLOGY. **CodeOntology organization** (repositories, last activity 2021–2022). Available at: https://github.com/codeontology. Accessed on: 2026-07-20.

[15] YAMAGUCHI, Fabian et al. **Modeling and Discovering Vulnerabilities with Code Property Graphs** (IEEE S&P 2014); overview: https://en.wikipedia.org/wiki/Code_property_graph. Accessed on: 2026-07-20.

[16] ABDELAZIZ, Ibrahim et al. **A Toolkit for Generating Code Knowledge Graphs (GraphGen4Code)**. arXiv:2002.09440. Available at: https://arxiv.org/pdf/2002.09440. Accessed on: 2026-07-20.

[17] PENG, Xin; WANG, Chong. **Code Digital Twin: A Knowledge Infrastructure for AI-Assisted Complex Software Development**. arXiv:2503.07967. Available at: https://arxiv.org/abs/2503.07967. Accessed on: 2026-07-20.

[18] LINKEDIN ENGINEERING. **Building the LinkedIn Knowledge Graph**. Available at: https://www.linkedin.com/blog/engineering/knowledge/building-the-linkedin-knowledge-graph. Accessed on: 2026-07-20.

[19] SEMANTIC MEDIAWIKI. **Help:RDF export**. Available at: https://www.semantic-mediawiki.org/wiki/Help:RDF_export. Accessed on: 2026-07-20.

[20] MEDIAWIKI. **Wikibase/DataModel**. Available at: https://www.mediawiki.org/wiki/Wikibase/DataModel. Accessed on: 2026-07-20.

[21] PROFESSIONAL WIKI. **Managing Data in MediaWiki: SMW vs Wikibase vs Cargo**. Available at: https://professional.wiki/en/articles/managing-data-in-mediawiki. Accessed on: 2026-07-20.

[22] MEDIAWIKI. **Extension:Cargo — Cargo and Semantic MediaWiki**. Available at: https://www.mediawiki.org/wiki/Extension:Cargo/Cargo_and_Semantic_MediaWiki. Accessed on: 2026-07-20.

[23] DENDRON. **Schemas** and **Hierarchies**. Available at: https://wiki.dendron.so/notes/c5e5adde-5459-409b-b34d-a0d75cbb1052/. Accessed on: 2026-07-20.

[24] NEO4J LABS. **neosemantics (n10s) — RDF for Neo4j** (repository). Available at: https://github.com/neo4j-labs/neosemantics. Accessed on: 2026-07-20.

[25] NEO4J. **neosemantics — Importing Ontologies**. Available at: https://neo4j.com/labs/neosemantics/4.3/importing-ontologies/. Accessed on: 2026-07-20.

[26] ALOCCI, Davide et al. **Property Graph vs RDF Triple Store: A Comparison on Glycan Substructure Search** (PLOS ONE, 2015). Available at: https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0144578. Accessed on: 2026-07-20.

[27] KUZUDB. **Kuzu — an embeddable property graph database** (repository, archived 2025-10-10). Available at: https://github.com/kuzudb/kuzu. Accessed on: 2026-07-20.

[28] APACHE JENA. **TDB2 Documentation**. Available at: https://jena.apache.org/documentation/tdb2/. Accessed on: 2026-07-20.

[29] FOWLER, Martin. **Ubiquitous Language**. Available at: https://martinfowler.com/bliki/UbiquitousLanguage.html. Accessed on: 2026-07-20.

[30] EVANS, Eric. **Domain-Driven Design Reference**. 2015. Available at: https://www.domainlanguage.com/wp-content/uploads/2016/05/DDD_Reference_2015-03.pdf. Accessed on: 2026-07-20.

[31] PAN, Tian. **GraphRAG vs Vector RAG: When Knowledge Graphs Outperform Semantic Search**. 2026. Available at: https://tianpan.co/blog/2026-04-17-graphrag-vs-vector-rag-knowledge-graphs. Accessed on: 2026-07-20.

[32] ONTOTEXT / DZONE. **RDF Triple Stores vs. Labeled Property Graphs: What's the Difference?**. Available at: https://dzone.com/articles/rdf-triple-stores-vs-labeled-property-graphs-whats. Accessed on: 2026-07-20.

[33] MININI, Alan et al. **Lifting EMMeT to OWL: Getting the Most from SKOS** (OWLED 2015). Available at: https://cgi.csc.liv.ac.uk/~valli/OWLED2015/OWLED_2015_paper_9.pdf. Accessed on: 2026-07-20.

[34] LIBRARY OF CONGRESS. **MADS/RDF (Metadata Authority Description Schema in RDF)**; and SUMMERS, Ed et al. **LCSH, SKOS and Linked Data** (arXiv:0805.2855). Available at: https://www.loc.gov/standards/mads/rdf/ and https://arxiv.org/pdf/0805.2855. Accessed on: 2026-07-20.

[35] EUROVOC / LDK 2025. **EuroVoc as SKOS Linked Data** (scale metrics). Available at: https://aclanthology.org/2025.ldk-1.34.pdf. Accessed on: 2026-07-20.

[36] MENDEZ, David et al. **Establishing Traceability between Natural Language Requirements and Software Artifacts** (ER 2024). Available at: https://model-engineering.info/publications/papers/ER24-Requirements2Code.pdf. Accessed on: 2026-07-20. `[unverified — single source: PDF fetch blocked, claim rests on search summary]`
