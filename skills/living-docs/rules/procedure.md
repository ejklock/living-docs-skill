# Procedure

## Authoring mechanics — CLI-first (hard rule)

The dividing line is **determinism**: a step with a single correct output given its inputs is
the CLI's job; the judgment prose (the "why") is yours to write directly in the file. Applied:

- **Use the CLI verb for every mechanical step — never hand-do it:** `new` (number + frontmatter
  + skeleton), `set <ref> <key> <value>` (set `status`/`description`/`owner`), `supersede <old>
  <new>` (links + status on both records), `index` (regenerate the listing), `check` (the gate,
  must pass).
- **One authoring path: scaffold, then edit the body.** `new` writes the numbered file with
  its frontmatter and title heading filled and every body section as a `{{SLOT: hint}}`
  placeholder; the hint says what belongs in the slot and vanishes with it. Replace every slot
  with prose, or delete the slot and its heading when the record has nothing to say there —
  `check` fails on any slot left behind.
- **Write the body prose directly.** The CLI must never author rationale, so there is no
  paragraph-editing verb — editing the body is a normal edit, not a process error. What *is* a
  process error is hand-numbering a doc, hand-writing frontmatter, hand-maintaining an index row,
  or hand-wiring `supersedes`/`superseded_by` when `supersede` does it deterministically. Write
  each paragraph as ONE line — never hard-wrap prose at a fixed column; the reading surface
  soft-wraps. `living-docs fmt` unwraps hard-wrapped paragraphs (ADR 0046), so a wrapped body is
  a `fmt` diff, not a style choice.
- **When a deterministic frontmatter mutation has no verb yet** and you keep doing it by hand,
  add it as a `living-docs set` key rather than normalizing the hand-edit.

## Setting up living docs in a new project

1. Create the project guide (`CLAUDE.md` or equivalent) with a **Docs index** section. Use `templates/claude-hard-rules.md` as the starting point for the hard-rules section; fill in the placeholders before committing.
2. Create `docs/` with the directories the project needs (`adr/`, `issues/`, and `prd/`/`research/`/`architecture/` as they earn their place). Seed `docs/constitution.md` from `templates/constitution.md`. Add the bundle-root `docs/index.md` (carrying `okf_version: "0.1"`), and give each directory its own `index.md` listing from day one — even if near-empty.
3. Seed the architecture views (`docs/architecture/`) with the high-level Mermaid diagrams the system already has, once it is worth drawing (`rules/architecture-diagrams.md`).
4. Record any already-made decisions as ADRs so they are not re-litigated — but **confirm each with the user before recording** (see *Adopting living docs in an existing project*, steps 2–4); never back-fill an ADR by inference alone.

## Adopting living docs in an existing project (brownfield)

An existing codebase already embodies decisions that were never written down. The failure mode here is the agent **back-filling ADRs by inference and presenting them as settled** — recording decisions the user was never asked to confirm. Adoption is therefore an *elicitation* exercise, not a transcription one.

1. **Scaffold without deciding.** Create `docs/` + each directory's `index.md` and the bundle-root `docs/index.md`. This is mechanical — no decisions are made here.
2. **Read the existing context first, then inventory the decisions — as candidates, not records.** Harvest what the project already carries: the code itself, plus the `README`, the agent guides (`CLAUDE.md` / `AGENTS.md`), package manifests, and any design notes. From that, produce a *list* of the load-bearing decisions the project appears to embody (stack, boundaries, data model, key trade-offs). Do **not** write ADRs yet.
3. **Present the inventory to the user and confirm each.** For every candidate, state the inferred decision and the alternatives it appears to have ruled out, and ask the user to confirm, correct, or discard it — grill the load-bearing ones (`grill-me` if installed). The user owns the decision; the agent only surfaces what the code implies.
4. **Record only the confirmed decisions as ADRs.** These are origin records — they supersede nothing. Capture the chosen option *and* the rejected alternatives the user confirmed. A candidate the user discards, or one whose rationale nobody actually knows, is **not** invented into an ADR.
5. **Seed the architecture views** with the high-level Mermaid diagrams the system already has, then resume the *Maintaining* loop below.

## Maintaining living docs (every task)

1. **Before coding:** read the relevant constitution and ADRs (`living-docs read` gives the in-force view; `read --contract` adds each record's contract sections). Decisions there are not to be re-opened casually. If a link or a search lands on a raw record whose body opens with a `SUPERSEDED` or `DEPRECATED` callout, treat it as history: follow the successor link or discard the record, never plan on it.
2. **While working:** if you make a decision with a load-bearing rationale that is expensive to reverse, **grill it before recording it** — surface the decision, ≥2 materially-distinct alternatives, and a recommendation to the user (run the `grill-me` companion if installed, else inline), then write an ADR capturing the chosen option *and* the rejected ones. A cheap, easily-reversed choice goes in the issue's body, not a new ADR. Never record a decision the user was not asked about.
3. **In the same change:** update every doc the structural change touches — index rows, architecture diagrams, the records that governed it. Run `living-docs check`.
4. **Never** leave an index stale, an orphan file unlinked, a diagram contradicting the code, or a superseded decision silently edited.
