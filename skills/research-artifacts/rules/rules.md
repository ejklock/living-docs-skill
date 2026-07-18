# Research rules & source discipline

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
