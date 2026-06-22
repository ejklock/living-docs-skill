---
type: Research
title: "Prior-art landscape: is Living Docs original or derivative?"
description: A sourced mapping of every part of Living Docs — the doc trail, the format and diagrams, the governance invariants — to its established prior art, and an honest assessment of what is genuinely novel.
status: Accepted
tags: [prior-art, originality, provenance, living-docs, documentation-as-code]
timestamp: 2026-06-15T00:00:00Z
---
# Prior-art landscape: is Living Docs original or derivative?

**Bottom line.** Almost everything in Living Docs already exists in industry, with a name and an originator. We are not copying one person or one project — we are **converging on (mostly re-implementing) a set of established, named documentation practices**. The defensible novelty is the **composition** plus one sharpened layer (the agent-enforceable governance invariants) — almost never the individual pieces. Full citations: the **References** section below.

## 1. The doc trail (constitution → PRD → ADR + BDR → issues → code)

- **ADR** is MICHAEL NYGARD (2011); the structured-markdown form is **MADR**; the *supersede-don't-delete* status convention is the **adr-tools** convention (PRYCE). Not novel rules.
- "**Living Documentation**" is CYRILLE MARTRAIRE's named methodology (2019) — we adopted an existing term, not coined it.
- **BDR ("Behavior Decision Record")** is a recent third-party coinage (ZANZAL, 2026, only months old), and is essentially **Specification by Example / Given-When-Then** (ADZIC, 2011; NORTH, 2006) wrapped in an ADR-style record. Treat it as an SbE record, not an industry standard or our invention.
- Neighbors we converge on: **Diátaxis** (PROCIDA), **arc42** (STARKE; HRUSCHKA), **C4** (BROWN), and the **docs-as-code** movement (Write the Docs).

**Verdict:** the trail itself is well-trodden. The only thing this research found *no prior assembly of* is the **invariant/governance layer** (supersede-never-delete + one-home-per-fact + indexed-or-it-doesn't-exist) carried in frontmatter as a fact contract and wired to a deterministic checker — that combination is unusual, not the trail.

## 2. The format and the diagrams

- **"Open Knowledge Format" is NOT ours.** It is a published **Google Cloud** spec (OKF v0.1, 2026-06-12) that matches our needs feature-for-feature — including the two tells (*reserved `index.md`*, *bundle-relative `/` links*). We adopt/vendor it; we cite Google. The spec is bundled verbatim under `skills/okf-knowledge-format/reference/`.
- **Mermaid** is the in-repo, text-based diagram syntax Living Docs uses for every living architecture, data-flow, and sequence view (SVEIDQVIST and the mermaid-js community). Living Docs only *uses* the syntax; it ships no Mermaid code.

**Verdict:** the format and the diagram language are adopted standards, credited to their owners — not authored here.

## 3. Has anyone assembled all of it?

No single project unites (a) the living-docs decision trail (constitution → PRD → ADR + BDR → issues), (b) a portable knowledge format with no-drift indexing, and (c) a single-source skill that installs across many AI coding harnesses. The closest:

- **GitHub Spec Kit** — the only other framework shipping a real `constitution.md` → spec → plan → tasks flow. No ADR/BDR split and no append-only governance invariants.
- **Agent OS** (Builder Methods) — closest to "indexed or it doesn't exist" with its *Index Standards*, but it has no supersede-don't-delete decision record.
- **BMAD-METHOD** — a strong brief → PRD → architecture → sharded-stories doc flow, but shallow cross-harness support and no ADR/BDR split or frontmatter fact contract.
- **wshobson/agents** and **ECC** — best on the cross-harness, single-source-compiled-to-many-harnesses model (architecturally near-identical to our install model), but flat skill collections with no enforced doc trail.

**Verdict:** the *union* of the trail + the format-with-no-drift-indexing + the cross-harness single source is genuinely unusual; each pillar individually is owned by someone else.

## 4. Honest novelty assessment

We invented no methodology. The originality is modest and concrete:

1. **The composition** — the living-docs trail + the OKF format + a cross-harness, single-source skill that installs into Claude Code, Cursor, Copilot, OpenCode, Codex, and Pi.
2. **The governance invariants** — *supersede-never-delete* + *one-home-per-fact* + *indexed-or-it-doesn't-exist*, carried in frontmatter as a fact contract **and wired to a deterministic checker** (`skills/living-docs/scripts/lint-docs.sh`). The prior-art research found no prior assembly of that exact governance layer.

The claim worth making is **agent-enforceable packaging of established practice**, not invention. Every claim of novelty should be framed as *"a particular defensible composition,"* never as inventing the underlying idea. The individual document types and the doc trail are well-trodden — and that is the right choice for a documentation discipline.

### Verification caveats

Several hosts returned HTTP 403 to direct fetch on 2026-06-15 (arXiv, BMAD/Agent OS docs); those claims were corroborated via multiple independent search extractions, not primary-source line-by-line reads. The ECC star count appears self-reported and is not relied upon. See the **References** section below for per-source links and access dates.

# References

Formatted per `skills/living-docs/rules/citation-conventions.md` (ABNT NBR 6023 structure, English labels, always the link). Alphabetical by first element. Access date for all online sources: 2026-06-15.

ADZIC, Gojko. **Specification by Example**: how successful teams deliver the right software. Shelter Island: Manning, 2011. Available at: https://gojko.net/books/specification-by-example/. Accessed on: 2026-06-15.

BMAD-CODE-ORG. **BMAD-METHOD**: Breakthrough Method for Agile AI-Driven Development. 2025. Available at: https://github.com/bmad-code-org/BMAD-METHOD. Accessed on: 2026-06-15.

BROWN, Simon. **The C4 model for visualising software architecture**. Available at: https://c4model.com/. Accessed on: 2026-06-15.

BUILDER METHODS. **Agent OS**. 2025. Available at: https://github.com/buildermethods/agent-os. Accessed on: 2026-06-15.

GITHUB. **Spec Kit**: spec-driven development toolkit. 2025. Available at: https://github.com/github/spec-kit. Accessed on: 2026-06-15.

GOOGLE CLOUD PLATFORM. **Open Knowledge Format — Specification v0.1 (Draft)**. 2026. Available at: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md. Accessed on: 2026-06-15.

MARTRAIRE, Cyrille. **Living Documentation**: continuous knowledge sharing by design. Boston: Addison-Wesley, 2019.

NORTH, Dan. **Introducing BDD**. 2006. Available at: https://dannorth.net/introducing-bdd/. Accessed on: 2026-06-15.

NYGARD, Michael. **Documenting Architecture Decisions**. 2011. Available at: https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions. Accessed on: 2026-06-15.

PROCIDA, Daniele. **Diátaxis**: a systematic framework for technical documentation authoring. Available at: https://diataxis.fr/. Accessed on: 2026-06-15.

PRYCE, Nat. **adr-tools**: command-line tools for working with Architecture Decision Records. Available at: https://github.com/npryce/adr-tools. Accessed on: 2026-06-15.

SHOBSON, Will. **agents**: a marketplace of subagents, skills, and commands for AI coding harnesses. 2025. Available at: https://github.com/wshobson/agents. Accessed on: 2026-06-15.

STARKE, Gernot; HRUSCHKA, Peter. **arc42**: template for architecture communication and documentation. Available at: https://arc42.org/. Accessed on: 2026-06-15.

SVEIDQVIST, Knut. **Mermaid**: generation of diagrams and flowcharts from text in a similar manner as Markdown. Available at: https://mermaid.js.org/. Accessed on: 2026-06-15.

ZANZAL, Owen. **Behavior Decision Records**: specifying what a system must do before deciding how to build it. Medium (DevOps<>AI), 2026. Available at: https://medium.com/devops-ai/behavior-decision-records-specifying-what-a-system-must-do-before-deciding-how-to-build-it-704876062688. Accessed on: 2026-06-15.
