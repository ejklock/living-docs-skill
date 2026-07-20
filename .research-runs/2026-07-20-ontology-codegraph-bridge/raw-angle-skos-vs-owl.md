# Angle: SKOS vs OWL sufficiency (agent aecf74, 9 tool_uses) — REFUTE F2

## SKOS data model [T1 W3C]
- skos:Concept = "an idea or notion; a unit of thought". ConceptScheme = aggregation of concepts. [T1] https://www.w3.org/TR/skos-reference/
- Labels: prefLabel/altLabel/hiddenLabel + notation ("string like 'T58.5' to uniquely identify a concept within a scheme"). hiddenLabel = accessible to text indexing/search. [T1] SKOS Primer https://www.w3.org/TR/skos-primer/
- Cross-scheme mapping relations native: exactMatch (transitive, "used interchangeably across wide range of IR apps"), closeMatch ("sufficiently similar, interchangeable in some IR apps"), broadMatch/narrowMatch (hierarchical), relatedMatch (associative). [T1] SKOS Reference

## The deliberate boundary [T1] — LOAD-BEARING
- "SKOS is not a formal knowledge representation language." Thesaurus/classification "does not assert any axioms or facts... do not have any formal semantics, cannot be reliably interpreted as either formal axioms or facts about the world." [T1] SKOS Reference
- Gap = class axioms, disjointness, cardinality/property restrictions, property characteristics (functional/inverse/transitive/symmetric), reasoner-based contradiction detection + auto-classification. [T1 academic] EMMeT-to-OWL OWLED 2015 https://cgi.csc.liv.ac.uk/~valli/OWLED2015/OWLED_2015_paper_9.pdf ; "SKOS with OWL: Don't be Full-ish!" https://ceur-ws.org/Vol-432/owled2008eu_submission_22.pdf
- EMMeT: partial lift to OWL driven by "an application (generating multiple choice questions) that requires more precision"; BUT "many controlled vocabularies do not need any such precision."

## SKOS/OWL tension [T1] — LOAD-BEARING
- skos:broader is NOT transitive by design; broaderTransitive = opt-in inference super-property for transitive closure. "Note that skos:broader is not a transitive property." [T1] SKOS Reference + Primer worked example (animals>mammals>cats)
- SKOS deliberately does NOT equate skos:Concept with owl:Class: "This specification does not make any additional statement about the formal relationship between the class of SKOS concepts and the class of OWL classes." Concepts = individuals, not owl:Class. [T1] SKOS Reference

## SKOS scales in production [T1-2]
- LCSH (id.loc.gov, maintained since 1898; first LC linked-data release Apr 2009 in SKOS/RDF). Getty AAT = SKOS + SKOS-XL + ISO 25964. EuroVoc v4.17 ~7,403 concepts / 202,008 prefLabels / 3.6M relations / 30 langs. AGROVOC 2023 ~40,983 concepts / 986,077 SKOS-XL prefLabels / ~5.9M relations / 58 langs. UNESCO Thesaurus v2 ~4,408 concepts.
- Sources: https://id.loc.gov/authorities/subjects.html ; https://www.getty.edu/research/tools/vocabularies/lod/ ; https://aclanthology.org/2025.ldk-1.34.pdf (EuroVoc) ; https://www.w3.org/2005/Incubator/lld/wiki/Use_Case_AGROVOC_Thesaurus ; UNESCO via aims.fao.org

## Cases of outgrowing SKOS [T1-2]
- LCSH -> MADS/RDF: SKOS "does not provide a way to capture the multiple components, and their types, of pre-coordinated subject headings" (e.g. "Drama--17th century" flattened to literal). MODELING-EXPRESSIVITY gap, NOT reasoning. [T1] https://www.loc.gov/standards/mads/rdf/ ; arxiv 0805.2855
- EMMeT + Digital Europa Thesaurus: lifted SKOS props to OWL object properties specifically for REASONING (consistency validation, infer implicit links, DL queries). [T1] OWLED 2015; [T3] medium Digital Europa Thesaurus; BestMap OWLED 2009 ceur-ws Vol-529

## Blocked/weak
- W3C WD "Using OWL and SKOS" (w3.org/2006/07/SWD/SKOS/skos-and-owl) fetched but extractor returned only generic pattern language; substituted authoritative SKOS Reference (Claim 3.2).

## VERDICT (angle): F2 REFUTED
SKOS is SUFFICIENT and is the purpose-built, industry-proven tool for a glossary linking doc concepts to code symbols with typed cross-scheme links. Every needed capability maps to a native SKOS construct: concepts + prefLabel/altLabel/hiddenLabel/notation (glossary terms), ConceptScheme (separate docs vocab from code-symbol vocab), mapping relations exactMatch/closeMatch/broadMatch/narrowMatch/relatedMatch (typed cross-scheme links) — same pattern LCSH/Getty/EuroVoc/AGROVOC run at millions of relations. W3C explicit SKOS "not a formal KR language" BY DESIGN; broader non-transitive w/ opt-in closure. SINGLE requirement forcing OWL = need to COMPUTE NEW FACTS BY LOGICAL ENTAILMENT (class axioms/disjointness/cardinality, reasoner contradiction detection or auto-classification, term-as-owl:Class subsumption w/ inheritance). Absent reasoning, SKOS is enough. Documented escalations all triggered by reasoning demand; the one non-reasoning escalation (LCSH->MADS) was internal-component-structure modeling, not reasoning.
