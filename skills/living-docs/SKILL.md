---
name: living-docs
description: Run a project's documentation as a living system — docs-first issues/PRDs, MADR-lite ADRs (supersede, never delete), Behavior Decision Records (BDRs), a project constitution, research artifacts, living Mermaid architecture diagrams, and semantic-index organization where every doc lands in exactly one place and indexes never drift. Use when setting up or maintaining project docs, writing an ADR/PRD/BDR/constitution/issue/research note, defining a term or acronym in the glossary, drawing or updating an architecture/flow/sequence diagram, splitting an oversized doc into an index, or enforcing the no-drift maintenance rule.
version: "0.7.0"
metadata:
  type: skill
  layer: procedural
  tags: [documentation, adr, prd, bdr, constitution, issues, research, architecture, semantic-index]
---

# Living Docs

Treat documentation as a living system that stays in sync with the code, not a write-once artifact that rots. The discipline has one spine — **every piece of knowledge has exactly one home, that home is indexed, and nothing structural ships without its doc** — and several document types that hang off it: a constitution, ADRs, BDRs, PRDs, issues, research, architecture diagrams, and a semantic context index.

This skill is stack-agnostic. It governs *how* docs are organized and maintained, never *what* technology a project uses.

---

## Using this skill (progressive disclosure)

This SKILL.md is a **slim stub** — a trigger plus a task->topic router. The `living-docs` CLI
holds the full, authoritative conventions and templates and discloses them progressively.
**Before authoring anything, load the topic for your task and operate from it, not from this
stub:**

- `living-docs skill living-docs --list` — discover every topic.
- `living-docs skill living-docs --topic <topic>` — load that topic's full rules (+ template).

Piped output is minified JSON (machine default); `--plain` for human text, `--json` to force
JSON. Topics: adr, prd, bdr, constitution, issue-workflow, glossary, architecture-diagrams,
semantic-index, doc-language, citation, procedure, enforcement-modes, check, okf-format,
doc-trail, size-targets, about (run --list for the full set).

---

## Core invariants (the spine)

These hold across every document type. Everything else is detail.

1. **Docs-first.** Author the body in the repo (`docs/…`) *before* publishing anywhere external (tracker, wiki). The repo file is the source of truth; the external copy is a mirror.
2. **One home per fact.** Each concept, decision, or requirement lives in exactly one file. No duplication — cross-reference instead of copying. Duplicated prose is drift waiting to happen.
3. **Indexed or it doesn't exist.** Every doc is reachable from an index (an `index.md` listing in its directory, and the bundle-root `docs/index.md` that the project guide links). No orphan files.
4. **Supersede, never rewrite history.** Decisions and requirements are append-only records. When something changes, mark the old record superseded and write a new one — never silently edit the past.
5. **No structural change without its doc.** New module, moved files, schema change, new data flow → update the relevant doc *and its diagram* in the same change. No "I'll document it later."

When in doubt, re-derive the right action from these five. The rules files below are just these invariants applied to each document type.

---

## When to invoke

- Standing up documentation for a project (creating `docs/` structure, the docs index, ADR/issue/BDR/constitution directories) → `living-docs skill living-docs --topic procedure`.
- **First time living-docs runs in a project** (no `## Living Docs` block in the project guide) → ask the enforcement-mode question and persist the answer → `living-docs skill living-docs --topic enforcement-modes`.
- **Adopting living-docs in an existing/brownfield project** (decisions already made but undocumented) → `living-docs skill living-docs --topic procedure` (*Adopting living docs in an existing project*): inventory the decisions, **confirm each with the user before recording any ADR**, never back-fill by inference alone.
- Writing or editing an **ADR** (an architectural/implementation decision) → `living-docs skill living-docs --topic adr`.
- Writing or editing a **PRD** (a product/feature requirement spec) → `living-docs skill living-docs --topic prd`.
- Writing or editing a **BDR** (observable behavior — inputs, outputs, Given/When/Then scenarios, **and the Test Design matrix for how each is tested**) → `living-docs skill living-docs --topic bdr`. A test-strategy *decision* (non-default level/technique, bar deviation) is an ADR `tags: [testing]`, not a new record type (no "TDR").
- Specifying a **non-functional requirement** (performance, availability, security, scale) → a **quality-attribute scenario** bound to an instrument in the **PRD** (`living-docs skill living-docs --topic prd`, rule 9); the decision + fitness function go in an ADR. Not a new doc type.
- Establishing or amending the **constitution** (foundational scope, data model, non-negotiables) → `living-docs skill living-docs --topic constitution`.
- Creating or editing an **issue/ticket** → `living-docs skill living-docs --topic issue-workflow`.
- Recording **research** (technology evaluation, external trade-offs) → load the **`research-artifacts`** skill. It owns the OKF research-note format (single file per note, no per-research subfolder), the source discipline, and the research → decision → issue traceable chain, and links back here for the ADR/BDR/issue artifacts. Pairs with the `deep-research` skill.
- Drawing or updating an **architecture, data-flow, or tool-calling diagram** → `living-docs skill living-docs --topic architecture-diagrams`.
- Defining a **term or acronym** the docs use → add it to the **glossary** (`docs/context/glossary.md`), one home per term → `living-docs skill living-docs --topic glossary`.
- A doc has grown too large or mixes concerns → **split into a semantic index** → `living-docs skill living-docs --topic semantic-index`.
- Sizing a record's body (aim ~100 lines, `check` advises at 120; research exempt; never trim a load-bearing rationale) → `living-docs skill living-docs --topic size-targets`.
- Enforcing the **no-drift maintenance rule** after any structural change → `living-docs skill living-docs --topic enforcement-modes` (refusal triggers) and `living-docs skill living-docs --topic procedure` (maintaining loop).
- Authoring or checking the **OKF format** of any doc (frontmatter `type`, reserved `index.md`/`log.md`, bundle-relative links, `# References`) → `living-docs skill living-docs --topic okf-format`.
- Deciding **which language** the docs are written in (default English; user may override at session start and pin it) → `living-docs skill living-docs --topic doc-language`.
- Understanding the **doc trail** (constitution → PRD → ADR/BDR → issues → code) or the **document map** (where each doc type lives) → `living-docs skill living-docs --topic doc-trail`.
- Understanding how this skill **composes** with `okf-knowledge-format`, `research-artifacts`, or optional companions, or its **provenance** → `living-docs skill living-docs --topic about`.
