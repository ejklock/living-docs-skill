# Living Docs

**Run a project's documentation as a living system — not a write-once artifact that rots.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Format: OKF](https://img.shields.io/badge/Format-OKF%20v0.1-blue.svg)](skills/okf-knowledge-format/reference/SPEC.md)
[![Skill: agent-ready](https://img.shields.io/badge/Skill-agent--ready-success.svg)](#whats-in-the-box)

Living Docs is an **agent skill** (works with Claude Code and any harness that
loads markdown "skills") that keeps a codebase's documentation in sync with its
code. It is stack-agnostic: it governs *how* docs are organized and maintained,
never *what* technology a project uses.

The whole discipline collapses to one spine:

> **Every piece of knowledge has exactly one home, that home is indexed, and
> nothing structural ships without its doc.**

Everything else — the constitution, ADRs, BDRs, PRDs, issues, research notes,
architecture diagrams, and the semantic context index — hangs off that spine.

---

## Why this exists

Most documentation rots because there is no contract keeping it honest. Living
Docs adds five governance invariants that an agent (or a human) can re-derive
every action from:

1. **Docs-first.** Author the body in the repo (`docs/…`) *before* publishing
   to any tracker or wiki. The repo file is the source of truth; the external
   copy is a mirror.
2. **One home per fact.** Each concept, decision, or requirement lives in
   exactly one file. Cross-reference instead of copying — duplicated prose is
   drift waiting to happen.
3. **Indexed or it doesn't exist.** Every doc is reachable from an `index.md`.
   No orphan files.
4. **Supersede, never rewrite history.** Decisions and requirements are
   append-only. When something changes, mark the old record superseded and
   write a new one — never silently edit the past.
5. **No structural change without its doc.** New module, moved files, schema
   change, new data flow → update the relevant doc *and its diagram* in the
   same change. No "I'll document it later."

These invariants are carried **in YAML frontmatter as a fact contract**, which
is the genuinely unusual part — see [Provenance](#provenance--honest-attribution).

---

## The doc trail

Every change follows one chain, from the foundational source of truth down to
code:

```mermaid
flowchart LR
  C[constitution] --> P[PRD]
  P --> A[ADR]
  P --> B[BDR]
  A --> I[issues]
  B --> I
  I --> K[code]
```

| Artifact | Role |
|---|---|
| **constitution** | Foundational source of truth: what the product is, core data model, non-negotiables. |
| **PRD** | What the system must do and why — feature/product requirement spec. |
| **ADR** | How the system is structured — architectural/implementation decision and rationale. |
| **BDR** | What the system must observably do — inputs, outputs, side effects, Given/When/Then scenarios. |
| **issues** | Execution slices — discrete units of work that trace back to ADRs/BDRs. |
| **code** | Implementation — every behavior, structure, and interface specified above, realized. |

---

## What's in the box

This repo bundles the Living Docs skill together with its two composition
dependencies and the prior-art research that backs its honesty claims:

| Path | What it is |
|---|---|
| [`skills/living-docs/`](skills/living-docs/) | The skill: the five invariants, the doc trail, per-doc-type conventions (`rules/`) and starter templates (`templates/`). |
| [`skills/okf-knowledge-format/`](skills/okf-knowledge-format/) | The **format** standard the docs use — Open Knowledge Format (OKF): markdown + YAML frontmatter, required `type`, reserved `index.md`/`log.md`, bundle-relative links. The OKF spec is **vendored verbatim** from Google Cloud Platform. |
| [`skills/research-artifacts/`](skills/research-artifacts/) | The research-note format and source discipline that feeds ADRs/PRDs (the `docs/research/` half of the trail). |
| [`references/prior-art-landscape.md`](references/prior-art-landscape.md) | The sourced prior-art analysis. Living Docs is **§2** of this broader study of the system it came from; it is included so every "credit, not invention" claim has a checkable citation. |

Each skill is self-describing — open its `SKILL.md` for the full operational
detail. Living Docs and OKF compose but do not overlap: **Living Docs governs
*which* docs exist and the no-drift discipline; OKF governs *how* a knowledge
bundle's markdown and frontmatter are shaped.**

---

## Installation

These are markdown skills — there is nothing to compile. Drop the skill
directories where your agent looks for skills.

### Claude Code (and compatible harnesses)

```bash
git clone https://github.com/ejklock/living-docs-skill.git
cd living-docs-skill
./install.sh                    # installs to ~/.claude/skills
# or target a custom directory:
./install.sh /path/to/skills
```

Restart your session so the skills are picked up, then invoke Living Docs by
asking your agent to set up or maintain docs (it triggers on writing an
ADR/PRD/BDR/constitution/issue, defining a glossary term, drawing an
architecture diagram, or enforcing the no-drift rule).

### Any other tool

Copy `skills/living-docs/`, `skills/okf-knowledge-format/`, and
`skills/research-artifacts/` into wherever that tool loads instructions from, or
just read the `SKILL.md` files — they are plain markdown meant to be read by
humans and agents alike.

---

## When to invoke

- Standing up documentation for a project (`docs/` structure, the docs index,
  ADR/issue/BDR/constitution directories).
- Writing or editing an **ADR**, **PRD**, **BDR**, **constitution**, or
  **issue** → load the matching `rules/` + `templates/` file.
- Recording **research** → the `research-artifacts` skill.
- Drawing or updating an **architecture / data-flow / sequence diagram**
  (living Mermaid, in-repo text that must match the code).
- Defining a **term or acronym** → the glossary, one home per term.
- A doc grew too large or mixes concerns → **split into a semantic index**.
- Enforcing the **no-drift maintenance rule** after any structural change.

---

## Composition with other skills

Living Docs is deliberately small and composes with the rest of your toolchain
rather than absorbing it: design grilling before a load-bearing ADR, an
architecture-improvement pass that reads the context index and ADRs, a
deep-research step that gathers the evidence `research-artifacts` then formats,
and an implementation-review step that checks code honors the ADRs/BDRs. See the
"Composition with other skills" section in
[`skills/living-docs/SKILL.md`](skills/living-docs/SKILL.md) for the full map.

> The design-grilling step composes with **`grill-me`** by
> **Matt Pocock** ([AI Hero](https://www.aihero.dev/)) — referenced, not bundled
> here. See [`ATTRIBUTION.md`](ATTRIBUTION.md).

---

## Provenance — honest attribution

**This work instrumentalizes established practices; it does not invent them.**
"Living documentation" is Cyrille Martraire's named methodology; ADRs are
Michael Nygard's (supersede-don't-delete is the adr-tools convention); BDRs wrap
Specification by Example / BDD (Adzic; North); the file format is Google Cloud
Platform's OKF, vendored verbatim. The only genuinely original part is the
**composition + the governance invariants** carried in frontmatter as a fact
contract.

Full credits and the per-source links are in
[`ATTRIBUTION.md`](ATTRIBUTION.md) and
[`references/prior-art-landscape.md`](references/prior-art-landscape.md).

---

## License

[MIT](LICENSE) © 2026 Evaldo Klock.

Vendored third-party content under `reference/` directories remains subject to
its own upstream license — see [`ATTRIBUTION.md`](ATTRIBUTION.md).
