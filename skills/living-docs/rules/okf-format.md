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
>   `okf_version`, bundle-relative links, the `# References` heading (§8). If OKF changes, only
>   this row moves — re-pin the version in the `okf-knowledge-format` skill and adjust.

Two rules apply to every concept file:

1. **Frontmatter with a required `type`.** Every non-reserved `.md` doc opens with a YAML frontmatter block whose `type` names the doc kind (`Constitution`, `PRD`, `ADR`, `BDR`, `Issue`, `Context`, `Architecture View`, `Research`, `Reference`). Recommended: `title`, `description`, `tags`, `timestamp`. Living-docs adds producer keys: `status`, `supersedes`, `superseded_by`, and an optional `visibility` (see rule 3). **Status moves into frontmatter — no `**Status:**` body line.**
2. **Reserved files + bundle-relative links.** The bundle root is `docs/`. Directory listings are `index.md` (OKF §6, no frontmatter — except the bundle-root `docs/index.md`, which carries `okf_version: "0.1"`). Optional `log.md` records directory history (§7). Cross-link with `/`-prefixed bundle-relative paths (`/adr/0007-slug.md`); list sources under a `# References` heading (§8), each entry formatted per `rules/citation-conventions.md` — **ABNT NBR 6023 structure, always carrying the link**, with connective labels in the project doc language (default English: `Available at: <URL>. Accessed on: <date>`) per `rules/doc-language.md`.
3. **Visibility (optional; default-deny).** A doc MAY carry `visibility: private | public | showcase`. **Absent ⇒ private** — omission can never publish by accident. It is the single, machine-readable declaration of what a future `public-export` may ship, made **once, at authoring time, by the human**, recorded as data — never re-judged by an LLM at publish. The author proposes a value (defaulting to private) and the human confirms; the *suggestion* is judgment, the *value* is mechanical. `living-docs check` validates the domain (a typo like `pubic` fails); deciding *which* docs are public is the author's, not the checker's. Friction sits on the dangerous direction: a private doc costs nothing, **elevating to `public`/`showcase` is the deliberate step.** Do not add `visibility` to templates — the omission is the intended private default. `index --visibility <csv>` lists only matching docs, so a public index never links a private doc.
