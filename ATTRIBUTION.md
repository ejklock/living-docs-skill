# Attribution

This repository **instrumentalizes established practices — it does not invent
them.** Living Docs is a *composition* of well-known documentation disciplines
plus a thin governance layer. Everything below is credited to its originator.
A full, sourced prior-art analysis lives in
[`references/prior-art-landscape.md`](references/prior-art-landscape.md).

## Vendored third-party content

| File | Source | Upstream license |
|---|---|---|
| `skills/okf-knowledge-format/reference/SPEC.md` | [GoogleCloudPlatform/knowledge-catalog](https://github.com/GoogleCloudPlatform/knowledge-catalog) — Open Knowledge Format (OKF) spec v0.1 | Governed by the upstream repository's license (Apache-2.0). Provenance and the pinned `sha256` are recorded in `skills/okf-knowledge-format/reference/SPEC.source.md`; refresh with `scripts/update-spec.sh`. |

The OKF spec is vendored **verbatim** so the format rules are usable offline and
diffable in version control. It is not authored here and is not covered by this
repository's MIT license.

## Methods this work composes (credit, not ownership)

- **"Living Documentation"** — Cyrille Martraire, *Living Documentation: continuous knowledge sharing by design* (Addison-Wesley, 2019). The name and methodology.
- **ADR (Architecture Decision Record)** — Michael Nygard, *Documenting Architecture Decisions* (2011). The structured-markdown form is **MADR**; the *supersede-don't-delete* status convention is the **adr-tools** convention (Nat Pryce).
- **BDR (Behavior Decision Record)** — an ADR-style wrapper (third-party coinage, Owen Zanzal, 2026) over **Specification by Example / BDD** (Gojko Adzic, 2011; Dan North, 2006).
- **OKF (Open Knowledge Format)** — Google Cloud Platform, *Open Knowledge Format — Specification v0.1* (2026).
- **Neighbors we converge on** — Diátaxis (Daniele Procida), arc42 (Gernot Starke; Peter Hruschka), C4 model (Simon Brown), and the docs-as-code movement (Write the Docs).

## Referenced, not bundled (credit where Living Docs composes with others)

Living Docs names a few neighbouring skills in its "Composition with other
skills" section. They are **not** shipped in this repo, but the people who
created them are credited here so no one is left uncredited:

- **`grill-me`** — Matt Pocock ([AI Hero](https://www.aihero.dev/)). The
  relentless design-interview skill Living Docs leans on before writing a PRD or
  a load-bearing ADR. The wording of the widely-shared `grill-me` skill
  originates from his AI Hero course; this project only *references* it.

## What is genuinely original here

Only the **composition** and the **governance invariants** — *supersede-never-delete*
+ *one-home-per-fact* + *indexed-or-it-doesn't-exist*, carried in frontmatter as a
fact contract. The prior-art research found no prior assembly of that exact
combination. The individual document types and the doc trail are well-trodden.

See [`references/prior-art-landscape.md`](references/prior-art-landscape.md) for
the per-source links, access dates, and the full honest novelty assessment.
