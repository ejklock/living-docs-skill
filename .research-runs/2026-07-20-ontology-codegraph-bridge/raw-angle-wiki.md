# Angle: Wiki + ontology prior art (agent ad531, 9 tool_uses) — REFUTE F4

## Semantic MediaWiki (SMW)
- Typed properties via [[Property::Value]] => subject-predicate-object triples, queryable. Property: namespace; type via "Has type". Categories -> OWL/RDFS: category-in-article = rdf:type; category-on-category-page = rdfs:subClassOf. Formally interpreted in OWL DL, OWL/RDF export. #ask/#show queries; subobjects.
- RDF export: per-page Special:ExportRDF, bulk dumpRDF.php (cronjob); external-vocab mapping (FOAF, Dublin Core); SPARQL endpoint.
- Scale: "used successfully even with millions of rows"; limiting factor = overly-complex queries / template formatting, NOT data volume. Enterprise: J&J, Pfizer, NASA, NATO, UN.
- [T1] semantic-mediawiki.org Help:Semantic_Web, Help:Properties, Help:RDF_export; meta.wikimedia Representation_in_database_and_RDF

## Wikibase (Wikidata engine)
- Data model: Entities (Items Q-IDs + Properties P-IDs) + Statements. Statements = snaks (value/novalue/somevalue) + qualifiers + references + ranks (Preferred/Normal/Deprecated). [T1] mediawiki.org Wikibase/DataModel
- Heavier than SMW. "Wikibase on its own does not allow you to visualize or query the data stored in your wiki." Needs separate SPARQL service. Self-host heavyweight ("requires a relatively powerful server"); access all-or-nothing. Bridge to SMW (Semantic Wikibase) is LOSSY (only main-snak value). [T2] Professional Wiki
- Fit: entity/fact-centric (atomic sourced statements), NOT document-centric. Poor fit for docs-per-project without heavy custom UI.

## Lighter tools
- Obsidian/Logseq: untyped links + backlinks; no native typed-relation/ontology layer ("typed links" = missing feature). Obsidian = doc-first page graph + Properties(YAML)+Dataview; Logseq = block outliner + Datalog query engine. Typed edges need plugins (Excalibrain, low perf at scale). [T3]
- TiddlyWiki/Foam: backlink/link-centric, no enforced typed hierarchy (Foam issue #604 requested schemas it lacks). [T1 Foam]
- Dendron: hierarchy as PRIMARY primitive (dot-delimited md) + OPTIONAL schema system = "type system for your notes" (autosuggest valid children, auto-template). Backlinks secondary. Refactor Hierarchy/Rename auto-updates links. [T1] wiki.dendron.so

## "One concept, one place" match
- Dendron = closest to single-authoritative-location semantic index ("one source of truth where a note can be filed"). Parallels living-docs' own "every doc lands in exactly one place". SMW/Wikibase do the OPPOSITE (concept = node reachable from many places via typed edges; many-to-many).
- Cargo (SMW's lighter sibling): structured data tied to TEMPLATES, stored in standard SQL tables, SQL queries, built-in FTS + auto-drilldown. "Cargo's querying ~30-50% faster than SMW's." Created because SMW judged too heavyweight/wrongly-grained. table-per-doc-type maps cleanly to projects->typed docs (ADR/PRD/BDR). [T1] mediawiki.org Extension:Cargo

## VERDICT (angle): F4 largely CONFIRMED (SMW/Wikibase don't fit well)
No drop-in match. Best structural template = HYBRID of Dendron (organization/authoring: hierarchy-primary, one canonical home, optional schemas, refactor) + Cargo (structured/queryable: template-anchored SQL tables + FTS). SMW = credible middle only if you need standards-based RDF/OWL export out of the box (cost = query/template perf). Wikibase = weakest fit (atomic statements not docs, no in-wiki authoring/query, heavy self-host, all-or-nothing access).
