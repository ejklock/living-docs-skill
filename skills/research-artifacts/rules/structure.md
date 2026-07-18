# Research structure & traceable chain

## Structure (OKF-conformant)

**One file per research session — never a per-research subfolder.** Each session is a single OKF `Research` concept at `docs/research/NNNN-<slug>.md` — a **sequential number leads the filename** (zero-padded, never reused, same scheme as ADRs/issues) and the **date lives in the frontmatter `timestamp`** (required), not in the filename. The file carries: question, query angles, findings (each with a confidence level and sources), contradictions, analysis, recommendations, and a trailing `# References` section (OKF §8) — **the home for that session's sources** (full NBR 6023 entries, always the link, key excerpts inline where a claim rests on a quote). See `templates/research-report.md`.

The top-level `docs/research/index.md` (OKF reserved listing) indexes every note with a one-line summary pointing to its file (`templates/research-index.md`). Cross-links are bundle-relative (`/research/NNNN-<slug>.md`).

### The general references roll-up

`docs/research/references.md` is a **cross-research bibliography roll-up** (`type: Reference`): the alphabetical union of every source cited across all notes, one NBR 6023 entry per source (always the link), each annotated with the note(s) that cite it (bundle-relative links). It is a **derived navigation index** — the answer to "where have we cited this before?" — **not a second home for the citation data**: on any divergence the per-note `# References` entry wins. Grown append-style — a note that adds a source not already listed adds it here in the same change. See `templates/research-general-references.md`.

---

## Research → decision → issue: the traceable chain

Every accepted recommendation must be traceable forward to a work item and backward to its evidence:

1. **Research → decision:** an accepted recommendation that changes architecture becomes an ADR; one that changes expected observable behavior becomes or amends a BDR (both authored via `living-docs`). Ask the user before decisions that change constitution-level positions.
2. **Decision → issue:** each ADR or BDR spawns one or more issues through the `living-docs` issue workflow. The issue links the ADR/BDR; the ADR/BDR links the research artifact.
3. **Chain completeness:** an orphan recommendation (not yet an ADR/BDR) is incomplete; an ADR/BDR without issues is unplanned; an issue without an ADR/BDR or research reference is ungrounded. All three gaps are valid review findings.

---

## Relationship to decisions

Research informs ADRs and BDRs. The typical flow:

1. A decision is unclear → run `deep-research` → artifacts land in `docs/research/`.
2. The decision is made → write an ADR (architecture/how) or BDR (observable behavior/what) via `living-docs`, whose Context links the research artifact.
3. Later, the research is reference material, not a requirement — it explains *why the decision looked right at the time*, even if a future ADR or BDR supersedes it.

---

## Anti-patterns

- Treating research as a living opinion doc that gets edited as views change. That destroys the audit trail. Each session is a dated snapshot; evolution is expressed as a *new* session, not an edit.
- Breaking the traceable chain by accepting a recommendation without writing its ADR/BDR, or writing a BDR/ADR without spawning issues, or opening issues without linking their source decisions.
