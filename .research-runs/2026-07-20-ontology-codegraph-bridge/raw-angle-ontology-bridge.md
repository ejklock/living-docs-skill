# Angle: Ontology-driven docs bridging code<->human (agent af5b23, 12 tool_uses)

## Ontology as intermediate representation (traceability)
- Ontology bridges "semantic gap" between NL docs/requirements and syntactic code; serves as intermediate artefact connecting metamodel-based + NL artefacts; independent of historical trace data. [T1] Springer OntoTrace 10.1007/978-3-031-29786-1_10
- Code-as-KG + graph+keyword+vector indexing + LLM/RAG improves requirement->code traceability over text-similarity alone. [T1] ER 2024 Requirements2Code (PDF fetch BLOCKED - FlateDecode; rests on search summary)
- Ontology+KG+LLM generalizable extraction pattern -> "semantic querying, traceability, automated reasoning". [T1] arxiv 2510.01409 OntoLogX
- Empirically validated ontology-based NLP tracing reqs->conceptual models. [T1] Requirements Eng journal 2025, 10.1007/s00766-025-00447-4

## Developer KGs in practice
- GraphGen4Code (MOST on-point): nodes = classes/functions/methods, edges include DOCUMENTATION links; "links library calls to documentation and forum discussions"; 79% linking accuracy (100% annotator agreement on 100 samples); 1.3M Python programs. [T1] arxiv 2002.09440
- Sourcegraph "code as data" global reference graph via SCIP (Protobuf, human-readable string symbol IDs, inspired by SemanticDB); cross-repo defs/refs; SCIP 10-20% smaller than LSIF. [T1] sourcegraph.com/blog/announcing-scip
- LinkedIn KG: trillions of edges; binary classifier per entity-relationship type; entity DISAMBIGUATION is the hard problem (the "uber" 96-member mismap). [T1] linkedin.com/blog/engineering
- Code Digital Twin (NORTH-STAR match): unifies code symbol graph (structural) + tacit/domain knowledge from docs+devs via domain-glossary construction + concept linking code symbols<->domain concepts. [T1] arxiv 2503.07967 (Xin Peng, Chong Wang). Verified abstract quote: "models both the physical and conceptual layers... integrating hybrid knowledge representations". Sharp "symbol graph vs domain KG" phrasing = paraphrase, needs full-text.
- Repo-level KG: FTS + vector + reverse-mapping from documentation nodes back to code nodes they describe. [T1] arxiv 2505.14394

## DDD ubiquitous language -> code identifiers
- Ubiquitous Language reaches "all the way into the product's source code"; class/method/var names mirror domain terms. [T1] Fowler; Evans DDD Reference: "a change in the language is a change to the model... refactor the code, renaming classes".
- CRUCIAL CONSTRAINT: mapping is BOUNDED-CONTEXT-scoped; same term differs across contexts; DDD explicitly REJECTS a single global unified model. => ontology bridge mappings must be context-qualified, NOT global. [T3 wiki DDD]
- Model-to-code tools (Context Mapper DSL, Actifsource, CubicWeb, OpenMDX) exist but are generation tools, not "glossary term<->existing code identifier" bridges.

## Semantic search / RAG: graph vs vector
- Pure vector better for topical "find content about X"; on simple semantic search vector RAG and GraphRAG comparable, graph adds overhead w/o benefit; graph wins often "cherry-picked worst case". [T2] tianpan.co
- Graph earns keep for: (a) multi-hop reasoning, (b) entity disambiguation via explicit edges, (c) temporal/causal, (d) synthesis across scattered chunks. [T2] flur.ee
- MS GraphRAG: entity/rel extraction -> Leiden community clustering -> community summaries -> global/local query. [T2]
- Graph costs: ontology/schema maintenance, hard incremental updates, latency (graph traversal 200-300ms vs sub-50ms vector ANN), query expertise. MEASURE FIRST: build 50-100 query eval set; "recall@k < 70% => retrieval is bottleneck". [T2] tianpan.co => maps to repo's own evidence-gate discipline.
- Hybrid = common prod answer: vector finds seeds, graph traversal enriches. [T2] memgraph HybridRAG

## Standards for cross-artifact concept links
- SKOS mapping properties (align concepts ACROSS schemes): exactMatch, closeMatch, broadMatch, narrowMatch, relatedMatch (sub-props of skos:mappingRelation). exactMatch transitive; closeMatch deliberately NON-transitive (prevents uncontrolled similarity propagation). Positioned as alternatives to overused owl:sameAs. Caveat: "no substitute for careful management of RDF graphs / provenance". [T1] W3C SKOS Reference https://www.w3.org/TR/skos-reference/ ; SKOS Primer https://www.w3.org/TR/skos-primer/
  => exactMatch/closeMatch/relatedMatch is the precise vocabulary for "code symbol <-> human concept" links of varying tightness across schemes (code graph vs human ontology).
- W3C Web Annotation Data Model: Annotation + Body(ies) + Target(s), JSON-LD; Target/Body may be a specific SEGMENT. "conveys that the body is related to the target". [T1] https://www.w3.org/TR/annotation-model/ ; Protocol on LDP https://www.w3.org/TR/annotation-protocol/
  => natural mechanism to attach human concept (Body) to precise code location/symbol (Target segment), interoperably.
- JSON-LD = shared serialization making SKOS + Web Annotation RDF-native yet dev-friendly (plain JSON that is also a graph).

## Blocked/gaps
- ER24-Requirements2Code.pdf: binary FlateDecode, no verbatim (rests on search summary).
- Code Digital Twin 2503.07967: sharp phrasing = paraphrase, only "physical+conceptual layers, hybrid knowledge representations" verbatim.
- Uber KG: no accessible tier-1 architecture writeup. LinkedIn = documented exemplar.

## VERDICT (angle)
Value of ontology bridge is CONDITIONAL and NARROW, not universal. Real+measured for: (1) entity disambiguation (explicit edges resolve term meaning where embeddings blur - LinkedIn "uber"; GraphGen4Code 79%); (2) multi-hop/relational + traceability queries (what req does this class satisfy / what breaks if I rename this concept) where FTS/vector degrade; (3) stable human-readable cross-artifact identity (SCIP string IDs, SKOS mapping relations - durable anchors vs per-model-drifting embeddings). For plain "find docs about X" the graph adds latency/schema-cost for no measurable gain -> measure query distribution first, expect hybrid (vector-seeded, graph-enriched). MINIMAL VIABLE SHAPE: code symbol graph w/ human-readable stable IDs (SCIP-style) on one side; lightweight human concept scheme (DDD glossary, bounded-context-scoped since global model rejected) on other; between them typed links via EXISTING standards - SKOS exactMatch/closeMatch/relatedMatch for concept-concept + W3C Web Annotation (Body=concept, Target=code segment) for concept-code-location, all JSON-LD so the bridge is itself a queryable graph. Build graph traversal only after eval corpus shows FTS+embeddings failing on relational/disambiguation queries you care about.
