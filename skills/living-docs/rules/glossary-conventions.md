# Glossary Conventions

The glossary is the project's single home for the **definition of every term and acronym** its docs use. It lives in the context bundle at `docs/context/glossary.md` (split into grouped files via `rules/semantic-index.md` once it outgrows ~200 lines). One term, one entry — every other doc *links* to the glossary instead of redefining.

## What goes in

- **Domain & product terms** the docs assume (the nouns of the system).
- **Engineering / methodology terms** that aren't common knowledge on the team (e.g. *characterization test*, *walking skeleton*, *mutation score*).
- **Acronyms & initialisms** (ADR, BDR, PRD, OKF, CC, …).

Leave out words any reader of the doc language already knows, and don't restate a definition that has a clearer home — link to it instead.

## Acronyms — headword as-is, expanded + explained in the doc language

1. **The headword is the acronym verbatim** — `ADR`, not `Architecture Decision Record (ADR)`. The abbreviation is what readers meet in the text, so it is the lookup key.
2. **Every acronym entry carries its expansion *and* a one-line definition.** The expansion may stay a proper noun in its original language (e.g. `OKF — Open Knowledge Format`); the **definition/explanation is written in the project doc language** (→ `rules/doc-language.md`, default English).
3. In a body doc, spell it out **once** on first use with the acronym in parentheses — `Architecture Decision Record (ADR)` — then use the acronym; the glossary is the durable home.

## Language (follows the doc-language rule)

- **Definitions are in the project doc language** — default English, or whatever is pinned per `rules/doc-language.md`.
- **Names stay in their original form.** You do not translate `ADR`, `BDR`, or `OKF`. Translate the *explanation*, never the *name*, the *identifier*, or the *acronym headword/expansion*.

## Format & order

- One entry per term: **headword** · (expansion, for acronyms) · definition (doc language) · optional *See also* cross-links.
- **Alphabetical by headword** for a flat glossary; **semantically grouped** (with an index) once it splits — the same machinery as the context vocabulary (`rules/semantic-index.md`).
- Start from `templates/glossary.md`.

## One home (no drift)

- A term is defined **once**, in the glossary. ADRs/PRDs/BDRs/issues that use it **link** to its glossary entry; they never carry a competing definition (the one-home-per-fact invariant).
- Vocabulary is **live**, not append-only: when a term's meaning changes, edit the entry and fix any now-wrong usages in the same change (don't supersede — that rule is for *decisions*, not definitions).

## Relationship to the context index

The glossary is part of the **context bundle** — it is the term/acronym layer of the domain & module vocabulary, not a second vocabulary. A small project keeps a single `docs/context/glossary.md`; a large vocabulary splits into grouped context files with the glossary as their alphabetical entry point. Either way there is exactly one home per term.
