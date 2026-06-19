---
type: Research
title: "Prior-art landscape: is this system original or derivative?"
description: Five-axis deep-research mapping every component of the repo (agent pipeline, living-docs trail, methodology skills) to its established prior art, and assessing what is genuinely novel.
status: Accepted
tags: [prior-art, originality, provenance, agent-pipeline, living-docs, methodology]
timestamp: 2026-06-15T00:00:00Z
---
# Prior-art landscape: is this system original or derivative?

**Bottom line.** Almost everything in this repo already exists in industry, with a name and an originator. We are not copying one person or one project — we are **converging on (mostly re-implementing) a large set of established, named practices**. No single project unites this exact combination, but two come strikingly close. The defensible novelty is the **composition**, plus ~2 sharpened molecules — almost never the individual pieces. Full citations: the **References** section below.

## 1. The agent pipeline (Architect → Coder → Reviewer)

The skeleton is not ours:

- The **Coder + Reviewer** role pair predates us in **ChatDev** (QIAN et al., 2023); the **PM → Architect → Engineer → QA** assembly line is **MetaGPT** (HONG et al., 2023) almost exactly. The difference is philosophical: we run a deliberately *de-committee-d* 3-role variant with **deterministic verdict-routing**, not agent dialogue.
- The "fixed **workflow** beats autonomous **agent**" thesis is the central distinction of ANTHROPIC's *Building Effective Agents* — our pipeline is a *prompt-chaining workflow* with an *evaluator-optimizer* tail (the Reviewer). Empirically backed by **Agentless** (XIA et al., 2024) and mini-SWE-agent ("less scaffold wins").
- "Own your control flow" is **12-Factor Agents**, Factor 8 (HORTHY).
- Expensive-plan / cheap-execute + model diversity per role is **Aider's architect/editor** mode (GAUTHIER, 2024) and Anthropic's productized `opusplan`.
- A stronger, **cross-family Reviewer** is applied **LLM-as-judge** (cross-family judging mitigates self-preference bias).

**Verdict:** every ingredient is prior art; the *3-role + anti-MAS + deterministic-routing + cross-family-reviewer* synthesis is a defensible composition, not an invention.

## 2. The living-docs trail (constitution → PRD → ADR + BDR → issues → code)

- **ADR** is MICHAEL NYGARD (2011); the structured-markdown form is **MADR**; the *supersede-don't-delete* status convention is the **adr-tools** convention (PRYCE). Not novel rules.
- "**Living Documentation**" is CYRILLE MARTRAIRE's named methodology (2019) — we adopted an existing term.
- Neighbors we converge on: **Diátaxis** (PROCIDA), **arc42** (STARKE; HRUSCHKA), **C4** (BROWN), and the **docs-as-code** movement (Write the Docs).

Two honesty corrections this research forced:

- **"Open Knowledge Format" is NOT ours.** It is a published **Google Cloud** spec (OKF v0.1, 2026-06-12) that matches our description feature-for-feature — including the two tells (*reserved `index.md`*, *bundle-relative `/` links*). We adopt/converge on it; we must cite Google.
- **"BDR — Behavior Decision Records" is a recent third-party coinage** (ZANZAL, 2026, ~2 months old), and is essentially **Specification by Example / Given-When-Then** (ADZIC, 2011; NORTH) wrapped in an ADR-style record. Treat it as an SbE record, not an industry standard or our invention.

**Verdict:** the trail itself is well-trodden. The only thing this research found *no prior assembly of* is the **invariant/governance layer** (supersede-never-delete + one-home-per-fact + indexed-or-doesn't-exist) carried in frontmatter as a fact contract — that combination is unusual, not the trail.

## 3. The methodology skills

Five of six are direct re-implementations of named practices:

| Skill | Instrumentalizes | Originator |
|---|---|---|
| `brownfield` | Characterization tests + legacy-change algorithm | FEATHERS (2004) |
| `greenfield` | Walking skeleton + vertical (Elephant Carpaccio) slicing | COCKBURN; KNIBERG; FREEMAN & PRYCE |
| `evidence-gate` | Fitness functions / hypothesis-driven dev | FORD; PARSONS; KUA (2017) |
| `tradeoff-analysis` | ATAM (Architecture Tradeoff Analysis Method) | KAZMAN et al., SEI |
| `parallel-trajectories` | Best-of-N sampling / self-consistency | WANG et al. (2022) |
| `adversarial-review` | Dialectical inquiry / devil's advocacy + LLM-as-judge | MASON & MITROFF (1981) |
| forces → three altitudes (architect-core standing rule + greenfield/brownfield architecture-method) | Architecture as a force-driven response (no best, only least-worst); quality-attribute/ASR-driven design; team/org as a force; patterns resolve forces; evolvability | RICHARDS & FORD; BASS, CLEMENTS & KAZMAN; CONWAY / SKELTON & PAIS; FOWLER / NEWMAN; ALEXANDER / POSA; FORD, PARSONS & KUA — full bundle `docs/research/0009-forces-drive-architecture-research.md` |

**Verdict:** the methods are not ours. Two molecules *are* sharpened: evidence-gate's **veto-before-build** pre-commitment, and adversarial-review's **independence + materiality gate**. The "forces → three altitudes" rule is pure instrumentalization (Richards & Ford et al.) — its only contribution is wiring the *connection* (force → altitude → tradeoff→recommend→select → ADR) into the standing pipeline. The rest is agent-native instrumentation of textbook practice.

## 4. Has anyone assembled all of it?

No single project unites (a) a fixed role-pipeline + (b) the living-docs decision trail + (c) a cross-harness, single-source composable-skills library. The closest:

- **BMAD-METHOD** — strongest on **(a)+(b)**: real PM/Architect/Dev/QA pipeline + brief→PRD→architecture→sharded-stories. Shallow cross-harness; no ADR/BDR split; no governance invariants.
- **ECC** and **wshobson/agents** — best on **(c)**: genuine adapter+core single-source compiled to many harnesses (architecturally near-identical to our install model). But flat skill collections, no routed pipeline, no enforced doc trail.
- **GitHub Spec Kit** — the only other framework shipping a real `constitution.md` → spec → plan → tasks. **Agent OS** — closest to "indexed or it doesn't exist" with its "Index Standards".

**Verdict:** the *union of all three pillars* is genuinely unusual; each pillar individually is owned by someone else.

## 5. Honest novelty assessment

We invented no methodology. The originality is: (1) **the union** (fixed-workflow pipeline + enforced living-docs trail + cross-harness SSOT skills), (2) **two sharpened molecules** (veto-before-build; independence+materiality review), and (3) the **mechanical-instrumentation discipline** (a constraint without an instrument is a vibe — mutation floors, CI regression floors, browser/pixel gates). Every claim of novelty should be framed as *"a particular defensible composition,"* never as inventing the underlying idea.

### Verification caveats

Several hosts returned HTTP 403 to direct fetch on 2026-06-15 (arXiv, anthropic.com, BMAD/Kiro/Agent OS docs); those claims were corroborated via multiple independent search extractions, not primary-source line-by-line reads. The ECC star count appears self-reported and is not relied upon. See the **References** section below for per-source links and access dates.

# References

Formatted per `skills/living-docs/rules/citation-conventions.md` (ABNT NBR 6023 structure, English labels, always the link). Alphabetical by first element. Access date for all online sources: 2026-06-15.

ADZIC, Gojko. **Specification by Example**: how successful teams deliver the right software. Shelter Island: Manning, 2011. Available at: https://gojko.net/books/specification-by-example/. Accessed on: 2026-06-15.

ANTHROPIC. **Building Effective Agents**. 2024. Available at: https://www.anthropic.com/research/building-effective-agents. Accessed on: 2026-06-15.

BROWN, Simon. **The C4 model for visualising software architecture**. Available at: https://c4model.com/. Accessed on: 2026-06-15.

FEATHERS, Michael. **Working Effectively with Legacy Code**. Upper Saddle River: Prentice Hall, 2004.

FORD, Neal; PARSONS, Rebecca; KUA, Patrick. **Building Evolutionary Architectures**: support constant change. Sebastopol: O'Reilly, 2017.

FREEMAN, Steve; PRYCE, Nat. **Growing Object-Oriented Software, Guided by Tests**. Boston: Addison-Wesley, 2009.

GAUTHIER, Paul. **Separating code reasoning and editing**. Aider, 2024. Available at: https://aider.chat/2024/09/26/architect.html. Accessed on: 2026-06-15.

GITHUB. **Spec Kit**: spec-driven development toolkit. 2025. Available at: https://github.com/github/spec-kit. Accessed on: 2026-06-15.

GOOGLE CLOUD PLATFORM. **Open Knowledge Format — Specification v0.1 (Draft)**. 2026. Available at: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md. Accessed on: 2026-06-15.

HONG, Sirui et al. **MetaGPT**: Meta Programming for a Multi-Agent Collaborative Framework. 2023. Available at: https://arxiv.org/abs/2308.00352. Accessed on: 2026-06-15.

HORTHY, Dexter. **12-Factor Agents**: principles for building reliable LLM applications. HumanLayer, 2024. Available at: https://github.com/humanlayer/12-factor-agents. Accessed on: 2026-06-15.

KAZMAN, Rick; KLEIN, Mark; CLEMENTS, Paul. **ATAM: Method for Architecture Evaluation** (CMU/SEI-2000-TR-004). Pittsburgh: Software Engineering Institute, 2000. Available at: https://www.sei.cmu.edu/documents/629/2000_005_001_13706.pdf. Accessed on: 2026-06-15.

MARTRAIRE, Cyrille. **Living Documentation**: continuous knowledge sharing by design. Boston: Addison-Wesley, 2019.

MASON, Richard O.; MITROFF, Ian I. **Challenging Strategic Planning Assumptions**: theory, cases, and techniques. New York: Wiley, 1981.

NORTH, Dan. **Introducing BDD**. 2006. Available at: https://dannorth.net/introducing-bdd/. Accessed on: 2026-06-15.

NYGARD, Michael. **Documenting Architecture Decisions**. 2011. Available at: https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions. Accessed on: 2026-06-15.

OUSTERHOUT, John. **A Philosophy of Software Design**. Palo Alto: Yaknyam Press, 2018.

PROCIDA, Daniele. **Diátaxis**: a systematic framework for technical documentation authoring. Available at: https://diataxis.fr/. Accessed on: 2026-06-15.

PRYCE, Nat. **adr-tools**: command-line tools for working with Architecture Decision Records. Available at: https://github.com/npryce/adr-tools. Accessed on: 2026-06-15.

QIAN, Chen et al. **Communicative Agents for Software Development** (ChatDev). 2023. Available at: https://arxiv.org/abs/2307.07924. Accessed on: 2026-06-15.

STARKE, Gernot; HRUSCHKA, Peter. **arc42**: template for architecture communication and documentation. Available at: https://arc42.org/. Accessed on: 2026-06-15.

WANG, Xuezhi et al. **Self-Consistency Improves Chain of Thought Reasoning in Language Models**. 2022. Available at: https://arxiv.org/abs/2203.11171. Accessed on: 2026-06-15.

XIA, Chunqiu Steven et al. **Agentless**: Demystifying LLM-based Software Engineering Agents. 2024. Available at: https://arxiv.org/abs/2407.01489. Accessed on: 2026-06-15.

ZANZAL, Owen. **Behavior Decision Records**: specifying what a system must do before deciding how to build it. Medium (DevOps<>AI), 2026. Available at: https://medium.com/devops-ai/behavior-decision-records-specifying-what-a-system-must-do-before-deciding-how-to-build-it-704876062688. Accessed on: 2026-06-15.

---

## All-in-one frameworks referenced (provenance for §4)

BMAD-CODE-ORG. **BMAD-METHOD**: Breakthrough Method for Agile AI-Driven Development. 2025. Available at: https://github.com/bmad-code-org/BMAD-METHOD. Accessed on: 2026-06-15.

BUILDER METHODS. **Agent OS**. 2025. Available at: https://github.com/buildermethods/agent-os. Accessed on: 2026-06-15.

SHOBSON, Will. **agents**: a marketplace of subagents, skills, and commands for AI coding harnesses. 2025. Available at: https://github.com/wshobson/agents. Accessed on: 2026-06-15.
