# Angle: RDF vs property graph tradeoffs (agent a9d44, 10 tool_uses) — REFUTE F3

## Model differences
- RDF = edge-centric triples (S-P-O) + global URIs + SPARQL (W3C std). LPG = node-centric, inline properties on nodes AND edges + Cypher/GQL (ISO GQL). [T2] DZone/Ontotext; Neo4j blog (pro-LPG bias)
- LPG attaches properties directly to edges; RDF cannot natively -> reification (extra nodes/triples). "relationships in RDF don't really exist as first class citizens". RDF-star/RDF 1.2 = convergence giving RDF edge-properties.
- RDF strengths: standards/interop, ontologies, inference/reasoning. LPG strengths: simplicity, edge properties, traversal ergonomics.
- [T1 peer-reviewed] PLOS ONE "Property Graph vs RDF Triple Store: Glycan Substructure Search" https://journals.plos.org/plosone/article?id=10.1371/journal.pone.0144578 : SPARQL-on-Neo4j-via-plugin ran 2.4-27x SLOWER than Jena Fuseki (itself slowest triple store) => paradigm-bridging is a perf tax.

## Neo4j + neosemantics (n10s)
- Imports/exports RDF into Neo4j LPG losslessly; maps OWL/RDFS/SKOS; SHACL. [T1] github.com/neo4j-labs/neosemantics (Labs = community-supported, not core)
- Mechanics: dataType props -> node properties; object props -> relationships; rdf:type -> node labels; every node has uri property. Requires unique-URI constraint + GraphConfig.
- LIMITATION: ontology loader only processes rdfs:domain/range + owl:Restriction; "All other elements will be ignored by this loader" => OWL semantics beyond subset dropped.
- Java (81.8%), runs inside Neo4j server (JVM) = separate service, NOT in-process for Rust.

## Embeddable/in-process stores
- Oxigraph: Rust RDF/SPARQL, in-process, RocksDB on-disk + in-memory. Native Rust crate. [T1]
- RDFLib: pure Python, in-process (not Rust). SQLite backends stale (rdflib-sqlite MOTHBALLED).
- Apache Jena TDB2: Java, embedded but JVM-only. COW MVCC.
- SQLite-as-triplestore: 3-col triples table; SPARQL = expensive self-joins; needs full SPO permutation indexes; NO off-the-shelf SQLite->SPARQL engine (hand-roll query logic). [T1/2 arxiv 1801.00036]
- KuzuDB: embeddable LPG + Cypher, has Rust bindings (cargo add kuzu), historically RDF->PG mapping. BUT ARCHIVED Oct 10 2025 (read-only, on-disk format never stabilized). [T1] github.com/kuzudb/kuzu

## JVM/separate-service cost
- Jena/RDF4J/n10s pull in JVM; Neo4j+n10s adds out-of-process server + serialization per query. No tier-1 benchmark of JVM-vs-Rust overhead (architectural fact). Oxigraph + SQLite ext both link as native Rust crates (single binary, in-process, no socket).

## codegraph-style (already SQLite): 3 options
- Extend SQLite: in-process, zero new runtime; BUT inherit self-join perf problem + no SPARQL/reasoning free.
- Add embedded RDF (Oxigraph): stays Rust in-process, real SPARQL 1.1 + RDF/SKOS/OWL ingestion; cost = 2nd store to sync + "not optimized yet" caveat.
- Separate graph DB (Neo4j+n10s): mature Cypher + RDF bridge, but out-of-process JVM, lossy OWL, extra service. Kuzu = archived risk.

## VERDICT (angle): F3 REFUTED for this use
Embedded RDF holds up; property-graph service does NOT clearly win. Only credible in-process same-language options = Oxigraph or extend SQLite (single binary, zero new runtime). Kuzu (embeddable LPG) archived. Neo4j+n10s buys Cypher + lossless RDF round-trip but imposes JVM + separate server + lossy OWL; SPARQL-on-LPG 2.4-27x slower even before cross-process hop. A "documentation ontology" with SKOS/OWL semantics is RDF's home turf. Least-cost path w/ real ontology semantics = Oxigraph embedded alongside (or projected from) existing SQLite graph. Non-Rust service warranted only if you need heavy Cypher analytics / Neo4j ops tooling — not implied by ontology-linking.
