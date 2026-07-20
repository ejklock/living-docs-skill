---
type: PRD
title: Living Docs Atlas — multi-project authoring wiki over living-docs-core
description: A web surface to navigate across projects and their docs, author/edit/create/delete/supersede through the deterministic core, search full-text, and (gated) surface a code-to-concept ontology bridge — evolving the read-only web front into an authoring wiki without breaking the determinism boundary or the one-language build.
status: Draft
superseded_by:
tags: [prd, atlas, web, authoring, wiki, multi-project, glossary, ontology, codegraph, search]
timestamp: 2026-07-20T14:45:52Z
---

# 1. Living Docs Atlas — Multi-Project Authoring Wiki over `living-docs-core`

## Problem / Motivation

Living Docs today is authored through the CLI and read through a **read-only** web view
(ADR [/adr/0006-web-read-only-axum.md](/adr/0006-web-read-only-axum.md)). Two pains follow
from that shape as the practice spreads across projects:

1. **Knowledge is siloed per repo and reachable only two ways** — the CLI or raw markdown.
   A person who does not live in a terminal cannot browse a project's decisions, author an
   ADR, or fix a stale doc. There is no single navigable surface across *projects*.
2. **The connective tissue is invisible.** The relationships that make a docs corpus a
   *system* — a term's definition, which decision superseded which, and eventually which
   human concept maps to which code symbol — exist only implicitly. You cannot see, from
   the docs, the line from a domain word down to the code that implements it, or back up.

The underlying need is a **navigable, human-authorable surface** over the corpus that keeps
every guarantee the CLI already gives (determinism, the doc-gate, "one doc, one place") and
adds the missing halves: authoring in the browser, navigation across projects, and a place
for the semantic layer (glossary now, code↔concept ontology later) to live.

This is an evolution, not a rewrite: `living-docs-core` already implements the deterministic
authoring verbs (`new`, `brief`, `supersede`, `index`, `export`, `check`), and the web front
already ships the three-pane, search-first, Cmd+K shell (ADR
[/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md](/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md)).
Atlas adds the **write path**, **multi-project navigation**, and the **semantic layer** on top.

## Goals

- Navigate **across projects** and, within a project, across its docs in the "one doc, one
  place" hierarchy — in the browser.
- **Author, edit, create, delete, and supersede** docs through the UI, reusing the
  deterministic core so browser authoring and CLI authoring produce identical, conformant records.
- Enforce the **doc-gate on write**: a change that would violate an invariant is rejected
  before it persists, in the browser just as `check` rejects it on disk.
- Provide **cross-project full-text search** (FTS5) as the primary find surface.
- Introduce a **glossary** layer (SKOS-shaped): a term defined once, in one place, with
  typed relations to other terms.
- Lay a **gated** path to a code↔concept ontology bridge over `codegraph` — built only when
  an evidence-gate justifies it (research
  [/research/0002-ontology-tooling-codegraph-docs-bridge.md](/research/0002-ontology-tooling-codegraph-docs-bridge.md)).
- Preserve the two locked constraints: the **determinism boundary** and **one language / one build**.

## Non-goals

- **No LLM inside the tool.** Atlas authors nothing; humans author, the tool validates and projects.
- **No OWL / reasoner** until a concrete logical-entailment requirement appears (research 0002, F2).
- **No non-Rust service** (Neo4j/JVM) and **no separate triple store** in the initial shape;
  the ontology begins as a *projection* of the existing SQLite records to RDF/JSON-LD.
- **Not a general-purpose wiki** (Notion/Confluence replacement). Scope is the living-docs
  doc types (ADR, BDR, PRD, issue, research, glossary).
- **No real-time multiplayer editing** (shared cursors / OT/CRDT) in v1.
- **No ontology graph-traversal / SPARQL layer** until the evidence-gate opens (research 0002, F5).

## Requirements

1. Atlas lists **registered projects** and opens one; each project renders its doc tree
   grouped by type, one canonical location per doc.
2. Reading a doc renders its markdown (pulldown-cmark) inside the three-pane shell + Cmd+K
   palette (ADR 0015), unchanged from today's read view.
3. **Create**: choosing a doc type + title in the UI invokes core `new`, producing a
   conformant scaffold; the created record passes `check`.
4. **Edit**: saving an edited doc writes through the store and runs `check`; a save that
   would violate an invariant (broken link, size, malformed frontmatter, bad Mermaid) is
   **rejected before persist** with the failing reason surfaced in the UI.
5. **Supersede**: the UI invokes core `supersede`, leaving both records linked and
   conformant (the ADR 0001 fitness function holds through the web path).
6. **Delete**: removing a doc updates indexes and leaves no dangling links — `check` stays green.
7. **Search**: cross-project full-text query (FTS5) returns ranked results, scopable to one
   project or all.
8. The **fs↔db sync / conflict contract** is explicit — which backend is authoritative on
   write, and how the other is reconciled — decided in an ADR **before** the write path ships
   (extends ADR [/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md](/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md),
   which covers lossless export but not write-time conflict).
9. **Glossary**: a `glossary` doc type where a term is a concept with `prefLabel`/`altLabel`
   and typed relations (`broader`/`narrower`/`related`), each term in exactly one place,
   validated by `check`.
10. **[Gated]** Typed links between glossary concepts and `codegraph` symbols are stored and
    projected to JSON-LD (SKOS mapping relations + W3C Web Annotation, Body = concept,
    Target = code segment). Built **only after** the evidence-gate opens (research 0002, F5/F6).

## Quality requirements (NFRs)

| Quality attribute | Scenario (source · stimulus · artifact · environment · response · measure) | Verified by |
|---|---|---|
| Determinism | The tool · projects records to index/RDF/JSON-LD · the projection artifacts · run twice on the same records · produces byte-identical output · 0 diff | Idempotency fitness function in CI (ADR 0001 style) |
| Integrity (doc-gate) | An author · saves a doc that violates an invariant · via the Atlas write path · in normal operation · the save is rejected and the record is unchanged · 100% of invalid saves blocked | `check` on write + browser-gate spec |
| Performance | A reader · requests a doc page · Atlas render path · on a bundle of ~1k docs · returns rendered HTML · p95 < 200 ms (measure first, then lock the floor) | Timing test + CI floor |
| One language / in-process | A maintainer · builds the workspace · Atlas + its stores · in CI · has no runtime dependency outside the Cargo workspace (no JVM/service) · 0 external runtimes | Dependency/build inspection (arch conformance) |
| Frontend correctness | A user · exercises create/edit/supersede/delete/search · Atlas UI · in a seeded browser session · the flow behaves as specified · all browser specs pass | Browser gate (Playwright specs under `tests/browser/`) |

## Acceptance criteria

- Creating an ADR end-to-end **through the Atlas UI** yields a file on disk that passes `living-docs check`.
- Editing a doc into an invalid state and saving shows the `check` failure and leaves the stored record untouched.
- Superseding doc A with doc B via the UI leaves both records present, mutually linked, and conformant.
- Deleting a doc removes it and its index entries with `check` still green (no dangling links).
- A term authored in project X is found by a cross-project search from project Y's view.
- The workspace builds and runs Atlas with no runtime outside the Rust workspace.

## Success metrics

- A new ADR/PRD can be authored, superseded, and found **without touching the CLI**.
- Share of doc operations (create/edit/supersede) performed via Atlas vs CLI trends up after launch.
- Median time-to-find a known doc across projects drops relative to the CLI/grep baseline.
- (Gated) Once the ontology bridge ships, relational/traceability queries ("what concept does
  this symbol implement", "what breaks if this term changes") are answerable in Atlas.

## Behavior (BDRs)

The observable flows each get a BDR (Given/When/Then + Mermaid) before their slice is built —
forthcoming, not yet created:

- Authoring lifecycle: create → edit → supersede → delete (one BDR per flow or a grouped one).
- Cross-project search.
- (Gated) Concept↔symbol link authoring and the code↔docs navigation it enables.

## Open questions

- **fs↔db write authority** — which backend wins on write, and how the other reconciles.
  Heads to an ADR (extends ADR 0007). Blocks Requirement 4/8.
- **Ontology substrate** — extend SQLite + project to JSON-LD, vs embed Oxigraph for SPARQL.
  Heads to an ADR, opened only when the evidence-gate opens (research 0002, F3b/F5).
- **Repo structure** — does Atlas's independent deploy cadence justify splitting the web front
  from the CLI (vs the locked monorepo)? Heads to an ADR; the wiki is the documented "reconsider when" trigger.
- **Multi-project registry** — how projects are registered/discovered/scoped in the db-store.
- **AuthN / AuthZ** — for a hosted, multi-project, writable surface, who may read and who may
  edit. Likely an ADR before any hosted deployment.

## Decision log

- Substrate/format findings that shape this PRD: research
  [/research/0002-ontology-tooling-codegraph-docs-bridge.md](/research/0002-ontology-tooling-codegraph-docs-bridge.md)
  (Rust-native SKOS-in-Oxigraph or SQLite-projection; wiki template = Dendron + Cargo, not SMW/Wikibase; ontology gated by evidence).
- ADRs resolving the open questions above — to be linked as they are decided.

## Delivery sequence (vertical, demoable slices)

Each slice is a thin end-to-end change; the ontology is deliberately last and gated.

- **A0 — fs↔db write-authority ADR.** Decide the sync/conflict contract (no code before the decision).
- **A1 — write path (edit).** Atlas can edit an existing doc; save runs `check` and blocks invalid writes. Smallest demoable win.
- **A2 — create + supersede + delete** through the UI, reusing core verbs.
- **A3 — multi-project registry + cross-project navigation and search.**
- **A4 — glossary doc type** (SKOS-shaped, `check`-validated). Human value with zero new infra.
- **A5 — [gated] evidence corpus** proving FTS+embeddings fall short on the relational/traceability queries that matter. Gate to A6.
- **A6 — [gated] code↔concept bridge** (typed links + JSON-LD projection; Oxigraph only if SPARQL is needed).

## Related

- Constitution: [/constitution.md](/constitution.md)
- Research: [/research/0002-ontology-tooling-codegraph-docs-bridge.md](/research/0002-ontology-tooling-codegraph-docs-bridge.md)
- ADRs: [/adr/0006-web-read-only-axum.md](/adr/0006-web-read-only-axum.md) · [/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md](/adr/0007-db-mode-authoring-data-model-and-lossless-export-contract.md) · [/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md](/adr/0015-web-ux-follows-the-three-pane-doc-site-archetype-with-search-first-cmd-k-palette.md)
