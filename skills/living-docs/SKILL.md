---
name: living-docs
description: Run a project's engineering decisions as a living log — MADR-lite ADRs (supersede, never delete) for decisions expensive to reverse, issues for the work (and its cheap-to-reverse choices), research artifacts, an optional PRD, a project constitution, and living Mermaid architecture views, where every record has exactly one home, indexes never drift, and a record is earned by materiality, not written per layer. Use when setting up or maintaining project docs, writing an ADR/PRD/constitution/issue/research note, drawing or updating an architecture diagram, or enforcing the no-drift maintenance rule.
version: "0.20.0"
metadata:
  type: skill
  layer: procedural
  tags: [documentation, adr, prd, constitution, issues, research, architecture]
---

# Living Docs

Living Docs is a **decision log with a gate**. It records the engineering decisions a future reader would pay to rediscover, traces each from its rationale to the code, and refuses to let a record exist unindexed, untyped, or silently rewritten. The spine — **every piece of knowledge has exactly one home, that home is indexed, and a material decision ships with its record** — carries a small set of record types: ADRs, issues, research, a constitution, an optional PRD, and living architecture views.

This skill is stack-agnostic. It governs *how* decisions are recorded and maintained, never *what* technology a project uses.

---

## Using this skill (progressive disclosure)

This SKILL.md is a **slim stub** — a trigger plus a task→topic router. The `living-docs` CLI
holds the full, authoritative conventions and templates and discloses them progressively.
**Before authoring anything, load the topic for your task and operate from it, not from this
stub:**

- `living-docs guide --list` — discover every topic.
- `living-docs guide <topic>` — load that topic's full rules (+ template).

Piped output is minified JSON (machine default); `--plain` for human text, `--json` to force
JSON.

Write ONLY the body below the closing ---. Frontmatter and indexes are CLI-owned: `living-docs set` / `supersede` / `index`.

- The spine invariants → `living-docs guide spine`.
- Authoring mechanics — CLI owns every deterministic step, you write only the prose →
  `living-docs guide procedure`.

## The one rule that decides whether to write a record

Write an **ADR** when a future reader would pay to rediscover *why* you chose this over the
alternatives — i.e. the decision is expensive to reverse. Otherwise put the choice in the
**issue** that carries the work. When in doubt, it is an issue. A record is earned by
materiality, never by the fact that a change touched structure or behavior — do not manufacture
a record per layer.

---

## When to invoke

- Standing up documentation for a project (creating `docs/` structure, the docs index, ADR/issue directories) → `living-docs guide procedure`.
- **Adopting living-docs in an existing/brownfield project** (decisions already made but undocumented) → `living-docs guide procedure`, *Adopting living docs in an existing project*: inventory the decisions, **confirm each with the user before recording any ADR**, never back-fill by inference alone. A bundle authored under an older organization is brought current by `living-docs index` and `living-docs fmt`, then `living-docs check`.
- Writing or editing an **ADR** (a decision expensive to reverse, with its rejected alternatives) → `living-docs guide adr` (load `guide procedure` first if not already loaded this session). A test-strategy *decision* (non-default level/technique, bar deviation) is an ADR `tags: [testing]`, not a new record type.
- Writing or editing a **PRD** (an optional product/feature spec: who asked, what is out of scope, what success looks like) → `living-docs guide prd` (load `guide procedure` first). A PRD without who-asked/out-of-scope is just a large issue — keep it an issue.
- Establishing or amending the **constitution** (foundational scope, non-negotiables) → `living-docs guide constitution` (load `guide procedure` first).
- Creating or editing an **issue/ticket** (the unit of work; it carries any cheap-to-reverse decision inline) → `living-docs guide issue-workflow` (load `guide procedure` first).
- Recording **research** (technology evaluation, external trade-offs) → load the **`research-artifacts`** skill. It owns the OKF research-note format, the source discipline, and the research → decision → issue traceable chain, and links back here for the ADR/issue artifacts. Pairs with the `deep-research` skill.
- Drawing or updating an **architecture, data-flow, or tool-calling diagram** → `living-docs guide architecture-diagrams`.
- A doc has grown too large or mixes concerns → **split into a semantic index** → `living-docs guide semantic-index`.
- **Reading the corpus as an agent** (what governs X *now*) → run `living-docs read` (active records only, supersede chains collapsed; `--topic <term>` to filter, `--contract` for the contract sections, `--full` for whole bodies — ADRs 0050, 0064), **never `index.md` directly**. A raw record whose body opens with a `SUPERSEDED` or `DEPRECATED` callout is history — follow the successor link or discard it, never plan on it.
- Sizing a record's body (aim ~100 lines, `check` advises at 120; research exempt; never trim a load-bearing rationale) → `living-docs guide size-targets`.
- Enforcing the **no-drift maintenance rule** after any structural change → run `living-docs check`; treat a non-zero exit as blocked; treat each advisory (`SIZE`, `LIVENESS stale-proposed`, `MOVED-SOURCE`) as work to schedule. Detail → `living-docs guide check`; the maintaining loop → `guide procedure`.
- Authoring or checking the **OKF format** of any doc (frontmatter `type`, reserved `index.md`/`log.md`, bundle-relative links, `# References`) → `living-docs guide okf-format`.
- Understanding the **doc trail** (constitution → PRD → ADR → issues → code) and which record type answers which question → `living-docs guide doc-trail`.
