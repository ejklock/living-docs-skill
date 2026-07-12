---
name: research-artifacts
description: Organize, format, and index research as OKF-conformant knowledge bundles — dated, sourced, append-only snapshots of the external evidence behind a decision (technology evaluations, library comparisons, industry-practice surveys). Use when recording research output, structuring a docs/research/ session, enforcing source discipline (primary sources, vendor-COI flags, inference-vs-fact labels, confidence levels), or wiring the research → decision → issue traceable chain. Pairs with deep-research (which gathers/verifies the evidence) and living-docs (which owns the ADR/BDR/issue artifacts research feeds into).
version: "0.5.0"
metadata:
  type: skill
  layer: procedural
  tags: [documentation, research, evidence, okf, sourcing, traceability]
---

# Research Artifacts

Research records the external evidence behind a decision: technology evaluations, library comparisons, industry-practice surveys. Research is **dated, sourced, and append-only** — a snapshot of what the evidence said at a point in time, not a living opinion.

This skill defines how research is *organized, formatted, and indexed*. The `deep-research` skill defines how it is *gathered and cross-verified*; `living-docs` owns the decisions and issues it feeds. The three compose. Research artifacts are **OKF concepts** — see the `okf-knowledge-format` skill for the frontmatter/reserved-file rules applied here.

---

## When to invoke

- Recording the output of a research session into `docs/research/`.
- Structuring or indexing a research note (`docs/research/NNNN-<slug>.md` + the index listing + the general roll-up).
- Enforcing source discipline on a draft (primary sources, vendor-COI flags, inference-vs-fact labels, confidence levels, fetch-failure notes).
- Wiring the research → decision → issue traceable chain (an accepted recommendation must reach an ADR/BDR and then issues, in `living-docs`).

---

## Structure (OKF-conformant)

**One file per research session — never a per-research subfolder.** Each session is a single OKF `Research` concept at `docs/research/NNNN-<slug>.md` — a **sequential number leads the filename** (zero-padded, never reused, same scheme as ADRs/issues) and the **date lives in the frontmatter `timestamp`** (required), not in the filename. The file carries: question, query angles, findings (each with a confidence level and sources), contradictions, analysis, recommendations, and a trailing `# References` section (OKF §8) — **the home for that session's sources** (full NBR 6023 entries, always the link, key excerpts inline where a claim rests on a quote). See `templates/research-report.md`.

The top-level `docs/research/index.md` (OKF reserved listing) indexes every note with a one-line summary pointing to its file (`templates/research-index.md`). Cross-links are bundle-relative (`/research/NNNN-<slug>.md`).

### The general references roll-up

`docs/research/references.md` is a **cross-research bibliography roll-up** (`type: Reference`): the alphabetical union of every source cited across all notes, one NBR 6023 entry per source (always the link), each annotated with the note(s) that cite it (bundle-relative links). It is a **derived navigation index** — the answer to "where have we cited this before?" — **not a second home for the citation data**: on any divergence the per-note `# References` entry wins. Grown append-style — a note that adds a source not already listed adds it here in the same change. See `templates/research-general-references.md`.

---

## Rules

1. **Single file, sequentially numbered, immutable.** Each session is one file `docs/research/NNNN-<slug>.md` — never a per-research subfolder. A **sequential number leads the filename** (zero-padded, never reused); the **date lives in the frontmatter `timestamp`** (required), not in the filename. Research is a snapshot — do not silently rewrite past findings. New evidence → a new note that references the old one.
2. **Every claim is sourced.** No factual claim without at least one URL. A claim is "verified" only with ≥2 independent sources; single-source claims are marked low-confidence. (See `deep-research` for the full confidence scale.)
3. **Recommendations are caveated.** A recommendation resting on low-confidence evidence must say so.
4. **Contradictions are surfaced, not hidden.** When sources disagree, record both positions with attribution.
5. **Indexed.** The `docs/research/index.md` pointer must match the note's filename exactly. No orphan research.
6. **Reference, don't inline.** ADRs and PRDs *link* to the research note (bundle-relative); they don't paste the findings. The note is the single home for the evidence.
7. **OKF format.** The note opens with YAML frontmatter carrying `type: Research` (the general roll-up carries `type: Reference`). Sources are listed under a `# References` heading. The `docs/research/index.md` listing carries no frontmatter.
8. **The note's `# References` is the home.** Every research note ends with a `# References` section (NBR 6023, always the link, excerpts inline where a claim rests on a quote) — the authoritative source list for that session.
9. **The general roll-up stays in sync.** A source added to a note is added to `docs/research/references.md` in the same change. The roll-up is a derived index, not a second home — on divergence the per-note entry wins.

---

## Source discipline

These rules are non-negotiable for every research session:

1. **Source-priority ladder (ordered, mandatory).** Prefer sources top-down, and back every core claim with the highest tier that plausibly exists for its topic: **(1) academic & primary** — peer-reviewed papers, preprints (arXiv/SSRN), official specs/standards, datasets, postmortems, benchmark reproducibles, court/regulatory records; **(2) authoritative secondary** — official vendor docs, standards-body explainers; **(3) general internet** — industry analysis, reputable news, well-sourced blogs; **(4) low-trust** — SEO/marketing/unsourced posts, used only to locate a primary source. A core claim resting only on tier 3+ where academic literature plausibly exists is marked low-confidence and the gap is flagged. (The `deep-research` skill searches the tiers in this order.)
2. **Flag vendor conflict-of-interest explicitly.** Mark every claim that comes from a vendor with `[COI: <vendor>]`. This applies in both directions: a vendor claim that favors the vendor is suspect; a vendor claim that cuts *against* the vendor's interest is a strength — note both. Unflagged vendor claims are disqualifying on review.
3. **Cross-check load-bearing numbers.** Any number a decision rests on must appear in ≥2 independent sources. Single-source numbers are marked low-confidence and must not be used as a basis for a P0 or P1 recommendation.
4. **Label inference vs documented fact.** Conclusions you derive from the evidence are inferences — mark them `[inference]`. Statements traceable to a source document are facts — cite the source. Never present an inference as a sourced fact.
5. **Drop or flag what cannot be corroborated.** A claim with one source and no independent corroboration is either dropped or included with an explicit `[unverified — single source]` marker and low confidence.
6. **Record fetch failures in the method note.** If fetches are blocked (paywalls, rate limits, access restrictions), record it in the report's Method section and adjust confidence ratings accordingly. Do not silently omit blocked sources.

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

---

## Composition with other skills

- **`deep-research`** — gathers and cross-verifies the evidence; this skill formats and indexes what it produces.
- **`okf-knowledge-format`** — the format standard these artifacts conform to (frontmatter `type`, reserved `index.md`, `# References`).
- **`living-docs`** — owns the constitution/PRD/ADR/BDR/issue artifacts. An accepted research recommendation flows into a `living-docs` decision and then issues; this skill provides the evidence those decisions cite.
