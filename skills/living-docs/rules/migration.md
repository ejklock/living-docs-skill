# Migration & Adoption

Adapting a project to the current Living Docs organization is a two-layer job (ADR 0037):
**detection is mechanical** — `living-docs migrate` scans and prints the plan — and **most
repair is judgment** the authoring model performs, record by record. The tool never edits
records; you do, through the CLI verbs.

## The one command

```bash
living-docs migrate [docs/]           # read-only; prints the ordered adaptation plan
living-docs migrate [docs/] --apply   # transactionally applies the mechanical subset (ADR 0040)
```

The advisor is safe to run anywhere, any time. `--apply` (fs-mode only) snapshots every
`.md` plus the seal ledger, runs `index` + `fmt`, and rolls back byte-for-byte on any
failure or `check` regression — then re-prints the remaining `AUTHOR` steps, which are
**never** applied automatically. An `ADOPT` plan refuses `--apply`: bootstrap is judgment
plus user confirmation. Each printed step carries a parseable prefix:

| Prefix | Meaning | Who acts |
|---|---|---|
| `RUN` | An exact mechanical CLI command (`index`, `fmt`, `hooks install`, `check`) | run it verbatim |
| `AUTHOR` | Judgment work on a named record — the gap and its governing ADR are stated | the authoring model, via CLI verbs + body edits |
| `ADOPT` | Bootstrap sequence for a project with no bundle at all | the authoring model, in the printed order |

Execute steps **in the printed order**, then re-run `living-docs migrate` until the plan
is empty ("Bundle is current — nothing to adapt.").

## Scenario 1 — legacy bundle → current organization

What the advisor detects, and how to repair each finding:

- **Root `architecture.md` (single file)** → one view per concern (ADR 0036): for each
  diagram, `living-docs new view "<name>" --kind <kind>`, move the Mermaid fence and its
  prose into the new record's body, state the view's drift instrument, then delete
  `architecture.md` and fix the root-index link. Load `--topic architecture-diagrams`.
- **View missing `kind:`** → set one of `context | container | component | flow |
  sequence | state | data-model | deployment` in its frontmatter (a freely editable key)
  so the generated index can rank it.
- **Non-Draft PRD with no `FR-N`/`NFR-N` IDs** → rewrite each requirement as an EARS
  statement under a stable ID and give NFR rows `NFR-N` IDs (ADR 0035); then make each
  covering BDR link the PRD and cite the IDs it proves. Load `--topic prd`. Respect
  PRD rule 6 (append-only once accepted): record the rewrite as an amendment, not a
  silent history edit.
- **Hand-maintained table index in a type directory** → just `living-docs index`; the
  generator migrates the listing in place, preserving the preamble.

## Scenario 2 — project entirely outside Living Docs

The `ADOPT` sequence bootstraps in dependency order: root `index.md` → constitution →
hooks → back-filled ADRs → first architecture view → `index` + `check`. Two hard rules
while executing it:

1. **Brownfield back-fill is interview, not inference.** Inventory the standing decisions
   from the code, then **confirm each with the user before recording any ADR** — never
   back-fill by inference alone (`--topic procedure`).
2. **Ask the enforcement-mode question first** if the project guide has no `## Living
   Docs` block, and persist the answer (`--topic enforcement-modes`).

## After migrating (either scenario)

Once the plan is empty, baseline provenance sealing: `living-docs seal init` (ADR 0039).
From then on `check` catches records created or owned-key-edited outside the CLI on this
clone; re-run `seal init` after any git merge/checkout that legitimately rewrites records.
Author follow-up records in one call each with `new <type> "<title>" --json '{...}'`
(ADR 0038) — section keys are the template's own headings.

## Anti-patterns

- Bulk-editing legacy records to "look current" without re-running `living-docs check` —
  the gate, not the eyeball, decides when migration is done.
- Back-filling ADRs from code inference without user confirmation.
- Treating `AUTHOR` steps as batch find-and-replace: each is a judgment pass over one
  record, with its governing ADR loaded.
