# Angle: Code knowledge graph standards (agent a5755, 10 tool_uses)

## LSIF (Microsoft)
- LSIF dumps language-server knowledge so LSP requests (defs/refs/hovers) answer without running the server; reuses LSP data types. Graph of vertices (documents, ranges, resultSets, hovers) + edges (LSP requests); cross-project via "monikers".
- [T1] https://microsoft.github.io/language-server-protocol/specifications/lsif/0.4.0/specification/ — LSIF Spec 0.4.0
- [T1] https://microsoft.github.io/language-server-protocol/overviews/lsif/overview/ — LSP/LSIF Overview

## SCIP (Sourcegraph)
- Protobuf schema centered on human-readable string symbol IDs (replaces LSIF monikers/resultSet). Transmission format, explicitly NOT a query/storage format; "does not aim to support efficient code navigation by itself". Influenced by Scala SemanticDB. Indexers: scip-java, scip-typescript, rust-analyzer, scip-clang, scip-python.
- [T1] https://sourcegraph.com/blog/announcing-scip
- [T1] https://github.com/scip-code/scip/blob/main/docs/DESIGN.md

## Kythe (Google)
- Language-agnostic graph: nodes, edges, facts. Facts = named bytestrings on nodes; edges = directed labelled; emitted as stream of entries; nodes id'd by 5-field VNames. Cross-refs via anchors: define/binding + ref edges. Schema extensible w/o central authority; reverse edges auto-generated. Has doc/uri fact but no NL-concept ontology.
- [T1] https://kythe.io/docs/schema-overview.html
- [T1] https://kythe.io/docs/kythe-storage.html
- [T1] https://kythe.io/docs/schema/writing-an-indexer.html

## Code Property Graph (Joern/ShiftLeft)
- Merges AST+CFG+PDG at shared statement/predicate nodes; labeled property graph (Neo4j/JanusGraph-style), NOT RDF. Yamaguchi et al., IEEE S&P 2014. Joern = OSS ref impl; ShiftLeft/Qwiet = commercial; Plume merged into Joern 2021. Security/dataflow oriented; no ontology/NL layer.
- [T1 paper] Yamaguchi et al., "Modeling and Discovering Vulnerabilities with Code Property Graphs", IEEE S&P 2014
- [T2] https://en.wikipedia.org/wiki/Code_property_graph

## CodeOntology (closest to the bridge)
- OWL 2 ontology of OO structural entities + parser serializing Java source/bytecode to RDF triples, SPARQL-queryable. Designed in Protégé. Links code entities to NL concepts by disambiguating doc comments against DBpedia via TagMe. => closest existing code-symbol<->NL-concept RDF bridge.
- MAINTENANCE: dormant. parser repo last updated Oct 2021; CodeOntologyPython Oct 2022; question-answering Jul 2022. Java-only.
- [T1] https://link.springer.com/chapter/10.1007/978-3-319-68204-4_2 — "CodeOntology: RDF-ization of Source Code" (ISWC 2017)
- [T1 repo] https://github.com/codeontology

## SEON (two projects, same acronym)
- (a) Software Evolution ONtologies: pyramid of OWL ontologies + Linked Data (se-on.org). Würsch et al., Computing 94(11):857-885 (2012). https://link.springer.com/article/10.1007/s00607-012-0204-1 [T1]
- (b) Software Engineering Ontology Network (NEMO/UFES Brazil): network of SE reference ontologies (reqs/design/coding/testing). Ruy et al., EKAW 2016. https://link.springer.com/chapter/10.1007/978-3-319-49004-5_34 [T1]
- Both are reference/domain ontologies, NOT per-repo symbol indexers.

## Traceability KGs (code<->doc linking research frontier)
- Traceability KGs = artifacts as typed nodes, trace links as typed directed edges (labeled property graph), each node has text + embedding, edges have confidence. [T2] arxiv 2606.17203
- NL<->code trace-link recovery treated as learned similarity/GNN problem (HGNNLink) because code lacks functional-semantic descriptions + high spurious textual similarity. arxiv 2509.05585; Springer 10.1007/s10515-025-00528-2
- Requirements traceability survey: connecting artifacts "requires a knowledge graph for domain concepts" + heuristics — must be built/supplied. [T1] https://arxiv.org/abs/2405.10845
- LLM doc-to-code traceability viable w/o formal ontology (best LLM F1 ~79-80% vs TF-IDF/BM25/CodeBERT). [T2] https://arxiv.org/abs/2506.16440

## VERDICT (angle)
No mature, maintained off-the-shelf code-ontology standard both maps repo symbols into a queryable graph AND bridges to human doc concepts in RDF/SKOS. Industrial standards (LSIF/SCIP/Kythe) = protobuf/property-graph, code-boundary only, no doc-concept/SKOS layer; SCIP disclaims being queryable store. CPG = security-focused property graph. CodeOntology = the one true code->RDF/OWL + doc->DBpedia bridge, but Java-only + unmaintained since 2021-22. SEON = domain reference ontologies, not symbol indexers. Frontier treats code<->NL link as build-or-learn (GNN/embeddings/LLM). => For code-symbol layer a purpose-built graph (SCIP/Kythe/codegraph-style) wins; for the code<->doc bridge nothing standard exists, so a hand-rolled RDF/SKOS map is NOT made redundant.
