# Procedure

## Authoring mechanics — CLI-first (hard rule)

The dividing line is **determinism**: a step with a single correct output given its inputs is
the CLI's job; the judgment prose (the "why") is yours to write directly in the file. Applied:

- **Use the CLI verb for every mechanical step — never hand-do it:** `new` (number + frontmatter
  + skeleton), `supersede <old> <new>` (links + status on both records), `index` (regenerate the
  listing), `check` (the gate, must pass), `export`/`brief` (byte-stable materialization /
  pre-filled scaffold).
- **Write the body prose directly.** The CLI must never author rationale, so there is no
  paragraph-editing verb — editing the body is a normal edit, not a process error. What *is* a
  process error is hand-numbering a doc, hand-writing frontmatter, hand-maintaining an index row,
  or hand-wiring `supersedes`/`superseded_by` when `supersede` does it deterministically.
- **When a deterministic frontmatter mutation has no verb yet** (e.g. set status, add a tag) and
  you keep doing it by hand, harden it into a new CLI verb rather than normalizing the hand-edit.

## Setting up living docs in a new project

1. **Ask the enforcement-mode question** (see *Enforcement modes* → *First-run question*) before anything else, since no preference is persisted yet. Record the answer as the `## Living Docs` block in the project guide; default to `strict` if the user has no preference.
2. Create the project guide (`CLAUDE.md` or equivalent) with a **Docs index** section and a **Maintenance rule** section (copy the wording from `rules/maintenance-invariant.md`). Use `templates/claude-hard-rules.md` as the starting point for the project guide's hard-rules section — it already carries the `## Living Docs` enforcement block; fill in the placeholders before committing.
3. Create `docs/` with the directories the project needs (`adr/`, `bdr/`, `issues/`, `prd/`, `research/`, `context/`). Seed `docs/constitution.md` from `templates/constitution.md`. Add the bundle-root `docs/index.md` (carrying `okf_version: "0.1"`), and give each directory its own `index.md` listing from day one — even if near-empty.
4. Seed the context index (`docs/context/index.md`) with whatever domain vocabulary exists, and the glossary (`docs/context/glossary.md`) with the terms and acronyms the docs already assume (`rules/glossary-conventions.md`). Grow both as concepts are named.
5. Seed the architecture doc (`docs/architecture.md`) with the high-level Mermaid views the system already has. Promote to a `docs/architecture/` directory + index once it grows (`rules/architecture-diagrams.md`).
6. Record any already-made decisions as ADRs so they are not re-litigated — but **confirm each with the user before recording** (see *Adopting living docs in an existing project*, steps 3–5); never back-fill an ADR by inference alone.

## Adopting living docs in an existing project (brownfield)

An existing codebase already embodies decisions that were never written down. The failure mode here is the agent **back-filling ADRs by inference and presenting them as settled** — recording decisions the user was never asked to confirm. Adoption is therefore an *elicitation* exercise, not a transcription one.

1. **Ask the enforcement-mode question** (first-run) and persist the `## Living Docs` block, exactly as for a new project.
2. **Scaffold without deciding.** Create `docs/` + each directory's `index.md`, the bundle-root `docs/index.md`, and seed the glossary/context index from vocabulary already present in the **code, the `README`, and the agent guides (`CLAUDE.md` / `AGENTS.md`)**. This is mechanical — no decisions are made here.
3. **Read the existing context first, then inventory the decisions — as candidates, not records.** Harvest what the project already carries: the code itself, plus the `README`, the agent guides (`CLAUDE.md` / `AGENTS.md`), package manifests, and any design notes or comments. From that, produce a *list* of the load-bearing decisions the project appears to embody (stack, boundaries, data model, key trade-offs). Do **not** write ADRs yet.
4. **Present the inventory to the user and confirm each.** For every candidate, state the inferred decision and the alternatives it appears to have ruled out, and ask the user to confirm, correct, or discard it — grill the load-bearing ones (`grill-me` if installed). The user owns the decision; the agent only surfaces what the code implies.
5. **Record only the confirmed decisions as ADRs.** These are origin records — they supersede nothing. Capture the chosen option *and* the rejected alternatives the user confirmed. A candidate the user discards, or one whose rationale nobody actually knows, is **not** invented into an ADR.
6. **Seed the architecture doc** with the high-level Mermaid views the system already has (`rules/architecture-diagrams.md`), then resume the *Maintaining* loop below.

## Maintaining living docs (every task)

1. **Before coding:** read the `## Living Docs` enforcement mode from the project guide (if the block is absent, this is the first run — ask the first-run question and persist the answer). Then read the relevant constitution, ADRs, BDRs, and the context index. Decisions there are not to be re-opened casually.
2. **While working:** if you name a new concept, add it to the context index; if you introduce a new term or acronym, define it once in the glossary. If you make a decision with a load-bearing rationale, **grill it before recording it** — surface the decision, ≥2 materially-distinct alternatives, and a recommendation to the user (run the `grill-me` companion if installed, else inline), then write an ADR capturing the chosen option *and* the rejected ones. If you specify observable behavior, write or amend a BDR. Never record a decision the user was not asked about (see *Enforcement modes → Mode governs completion, not elicitation*).
3. **In the same change:** update every doc the structural change touches — index rows, **architecture diagrams**, vocabulary. Run the maintenance checklist (`rules/maintenance-invariant.md`).
4. **Never** leave an index stale, an orphan file unlinked, a diagram contradicting the code, or a superseded decision silently edited.
