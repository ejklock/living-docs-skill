# Composition, notes & provenance

## Composition with other skills

**Bundled**, installed alongside living-docs:

- **`okf-knowledge-format`** — the OKF file-format standard (frontmatter `type`, reserved `index.md`/`log.md`, bundle-relative links, conformance rules). Living-docs governs *which* docs exist and the no-drift discipline; OKF governs *how* a knowledge bundle's markdown and frontmatter are shaped.
- **`research-artifacts`** — owns the research-note format and discipline (a single-file OKF `docs/research/NNNN-<slug>.md` with a trailing `# References`, source rules, the research → decision → issue chain). An accepted recommendation becomes an ADR/BDR here.

**Optional companions**, not included in this repo, composed with only if installed:

- **`grill-me`** — grills a PRD or load-bearing ADR before writing it, surfacing the decision, ≥2 materially-distinct alternatives, and a recommendation; without it, do the lightweight inline version.
- A **deep-research** skill gathers and cross-verifies the evidence that `research-artifacts` formats and indexes.
- A **codegraph** tool is the structural index of *code*, alongside living-docs' context index (concepts) and architecture diagrams (structure).

---

## Notes

- Keep individual docs scannable. When a context, vocabulary, or architecture file passes ~200 lines or mixes concerns, split it via `rules/semantic-index.md` rather than letting it grow.
- This skill describes conventions, not tooling. Numbering schemes (`NNNN`), tracker choice (GitHub Issues, Jira), and directory names can be adapted per project — the invariants must not.
- Templates in `templates/` are starting points. Trim sections that don't apply; never delete a section just to avoid filling it in if it's load-bearing.

---

## Provenance — instrumentalization, not invention

Almost every doc type here is established prior art — Living Documentation (Martraire), ADR/MADR
(Nygard; supersede-don't-delete via adr-tools/Pryce), BDR (Specification by Example — Adzic,
North), the problem-first PRD / vertical-slice issue workflow (Pocock; Cohn; Beck; Hunt & Thomas;
Cockburn), and the OKF format (Google Cloud Platform, v0.1) — this skill composes and enforces
them, it does not invent them. What it adds is the **explicit, agent-enforceable packaging**:
the governance invariants carried in frontmatter as a fact contract, wired to a deterministic
checker (`living-docs check`). Full citations: `../../references/prior-art-landscape.md`.
