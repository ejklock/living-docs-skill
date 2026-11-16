# Format: OKF-conformant

The five Core invariants (in the living-docs SKILL.md) govern *organization and lifecycle*; the **Open Knowledge Format** governs the *file format* so the corpus stays portable and agent-parseable. Every doc in the system is also an OKF concept. Load the `okf-knowledge-format` skill (it vendors the spec) when authoring or checking format.

> **OKF is a thin, swappable dependency — not a foundation (version risk).** OKF is **v0.1
> from a single vendor**; a backward-incompatible v0.2 is a real possibility. The five Core
> invariants do **not** depend on OKF — they depend only on a small set of frontmatter fields, so an
> OKF break cannot take the governance layer down with it. Keep the boundary explicit:
> - **Required by Living Docs** (the fact contract `living-docs check` enforces): a non-empty
>   `type`, and `status` + `superseded_by` on superseded records. These are *ours*; they survive
>   regardless of OKF.
> - **Inherited from OKF** (format conventions): reserved `index.md`/`log.md`, the bundle-root
>   `okf_version`, the `# References` heading (§8). Relative links between records are our own choice (not OKF-prescribed; see rule 2). If OKF changes, only
>   this row moves — re-pin the version in the `okf-knowledge-format` skill and adjust.

Two rules apply to every concept file:

1. **Frontmatter with a required `type`.** Every non-reserved `.md` doc opens with a YAML frontmatter block whose `type` names the doc kind (`Constitution`, `PRD`, `ADR`, `Issue`, `Context`, `Architecture View`, `Research`, `Reference`). Recommended: `title`, `description`, `tags`, `timestamp`. Living-docs adds producer keys: `status`, `supersedes`, `superseded_by`. **Status moves into frontmatter — no `**Status:**` body line.**
2. **Reserved files + relative links.** The bundle root is `docs/`. Directory listings are `index.md` (OKF §6, no frontmatter — except the bundle-root `docs/index.md`, which carries `okf_version: "0.1"`). Optional `log.md` records directory history (§7). Cross-link with paths relative to the linking file — `0001-foo.md` for a sibling, `../adr/0007-slug.md` across type directories. This is canonical because on GitHub a `/adr/…` link resolves against the repository root and returns 404 when the bundle sits under `docs/`, while a relative link resolves in the browser, in editors and in `check` (which still resolves the `/` form but does not prescribe it); list sources under a `# References` heading (§8), each entry formatted per `rules/citation-conventions.md` — **ABNT NBR 6023 structure, always carrying the link**, with connective labels in the project doc language (default English: `Available at: <URL>. Accessed on: <date>`) per `rules/doc-language.md`.
