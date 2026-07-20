# Angle: Rust RDF ecosystem maturity (agent a4a1, 24 tool_uses) — REFUTE F1

## Oxigraph
- Latest 0.5.9, published 2026-06-18; ~511,934 downloads. [T1] https://crates.io/api/v1/crates/oxigraph
- Near-monthly cadence: 0.5.7 (2026-04-19) ... 0.5.0 (2025-09-13), etc. [T1] CHANGELOG raw
- Most recent commit 2026-07-19 (day before report). [T1] https://api.github.com/repos/oxigraph/oxigraph/commits
- Open 112 / closed 247 issues (~0.45 ratio, healthy). ~1.8k stars. Lead maintainer Tpt (Thomas Tanon), offers paid support. Bus-factor: single lead.
- Features: SPARQL 1.1 Query+Update+Federated; XML/JSON/CSV/TSV results; RDF Dataset Canonicalization; preliminary SPARQL 1.2 behind sparql-12. RDF-star dropped for RDF 1.2 (0.5.0). JSON-LD 1.1 default (0.5.5). Formats: Turtle, TriG, N-Triples, N-Quads, RDF/XML, JSON-LD.
- Embeddable in-process Rust lib; RocksDB on-disk + in-memory (WASM default). Python + JS/WASM bindings + CLI.
- Users: Zazuko, RelationLabs, Data Treehouse, DeciSym.AI, ACE IoT.
- CAVEAT (README): "Oxigraph is in heavy development and SPARQL query evaluation has not been optimized yet." (perf caveat, not "don't use in prod").
- Sub-crates: oxrdf 0.3.3, oxttl 0.2.3, spargebra 0.4.6 (all 2026), oxrdfio/sparesults/spareval/sparopt.

## sophia
- Meta-crate 0.10.0 (2026-05-19); created 2018; ~458k downloads. Slow/irregular (~1 minor/yr) but current.
- Recent commit 2026-05-20. Open 22 / closed 99. 324 stars. Lead: Pierre-Antoine Champin. Bus-factor: single lead.
- Generic API for RDF 1.2. sophia_sparql = "(currently partial) implementation of SPARQL 1.2 Query" — INCOMPLETE, no Update. Formats: Turtle-family, JSON-LD 1.1, RDF/XML, N-Triples. In-memory only (sophia_inmem); NO native on-disk store. Has sophia_reasoner (Simple/RDF/RDFS entailment).
- Dependents: manas (Solid), nanopub.

## horned-owl
- 2.1.0 published 2026-07-17 (major bump 3 days before report). ~58.7k downloads. Rust 2024, needs Rust >=1.88.
- Recent commit 2026-07-20 (report date). Open 11 / closed 105. 98 stars. Lead: Phillip Lord (Newcastle Univ), academically backed.
- Full OWL 2 DL implementation + SWRL. Syntaxes: RDF/XML, OWL/XML, Functional (Pest parser), Manchester. 928 unit tests.
- Peer-reviewed: 20x-40x faster than Java OWL API; millions of terms. [T3 paper] Dagstuhl TGDK "Horned-OWL: Flying Further and Faster with Ontologies"
- NOT a reasoner (reasoning = companion whelk-rs). Bindings: py-horned-owl.

## Rust SKOS support — THE WEAK SPOT
- rdftk_skos: dormant. Newest 0.2.0 published 2021-06-14. Self-described "not a complete API... extensibility with OWL is limited." [T1] https://crates.io/api/v1/crates/rdftk_skos
- rdftk_core: more recent (~Nov 2024), added SKOS to PrefixMapping.
- OxiRS oxirs-rule ships skos_validation module (broader/broaderTransitive/top-concept checks). OxiRS = separate newer project. [T2] docs.rs/oxirs-rule
- NET: no actively-maintained dedicated SKOS crate. SKOS in Rust = build on Oxigraph/sophia w/ rdf_vocabularies, or dormant rdftk_skos, or emerging oxirs-rule.

## OxiRS (cool-japan/oxirs): SPARQL 1.2 + GraphQL + "AI reasoning", early-stage, not deep-verified. [T3]

## Blocked/partial fetches
- crates.io HTML for horned-owl (title only) -> worked around via JSON API.
- GitHub contributor graphs failed to render for sophia/horned-owl -> relied on commits/issues/stars.
- Some download counts / timestamps via WebFetch summarizer (accurate-to-source, not re-parsed).

## VERDICT (angle): F1 REFUTED (qualified yes)
Rust RDF ecosystem IS production-viable as embedded in-process lib. Oxigraph clearly strongest (embeddable, RocksDB persistent + in-memory, full SPARQL 1.1, JSON-LD/Turtle/etc, monthly cadence, active). Qualifications: query eval "not optimized yet"; single-maintainer bus-factor across all three. sophia = toolkit/library (partial SPARQL, no store). horned-owl = OWL modeling/parsing (active, fast), not a store/reasoner. SKOS weak spot: no maintained dedicated crate -> build SKOS on Oxigraph/sophia. Bottom line: embedded RDF/SPARQL store -> Oxigraph; add horned-owl if OWL in scope; don't rely on off-the-shelf SKOS crate.
