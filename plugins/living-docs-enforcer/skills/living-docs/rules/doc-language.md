# Documentation Language

The language of the doc corpus is a **project-wide non-negotiable**, not a per-file choice. Pick it once, pin it, follow it everywhere.

## The rule

1. **Default is English.** If the user has not declared a documentation language, author every doc — constitution, PRD, ADR, BDR, issues, research, `# References` prose — in **English**. This keeps the corpus portable and matches the default tooling/templates.
2. **The user may override at session start.** When the user names a documentation language while using the harness (e.g. "docs em português", "documentação em espanhol"), that language governs **all** docs from then on — not just the file in front of you.
3. **Pin it the moment it's chosen.** A declared language is a standing rule, so record it where it survives across sessions (re-read from disk, per the compaction model):
   - If the project has a **constitution**, add the language as a non-negotiable line there (`rules/constitution-conventions.md`).
   - If there is no constitution yet, write it into the **project guide** hard-rules section (`CLAUDE.md` or equivalent, via `templates/claude-hard-rules.md`).
   Once pinned, follow it without re-asking; the user does not have to repeat the choice every session.
4. **Never mix.** One corpus, one language. Don't write the PRD in one language and its ADRs in another. If the pinned language changes, that is a normal supersede event — new docs in the new language, old docs left as frozen history (invariant 4).

Terms and acronyms get their explanation in the pinned language — but the **name/headword stays as-is** — in the glossary; see `rules/glossary-conventions.md`.

## What stays language-invariant

These do **not** translate — they are structural identifiers, not prose:

- OKF frontmatter **keys** (`type`, `status`, `title`, …) and their controlled values (`type: ADR`, `status: Superseded`).
- The `# References` **heading** itself (OKF §8 reserved heading) and reserved filenames (`index.md`, `log.md`).
- Code, identifiers, slugs, and the ABNT NBR 6023 **structure** of each reference (CAPS surnames, **bold** title, ISO access date, always-the-link).

## Citations follow the doc language

The reference *structure* is fixed (NBR 6023), but the connective **labels** localize to the pinned language — see `rules/citation-conventions.md`:

| Doc language | URL label | Date label |
|---|---|---|
| English (default) | `Available at:` | `Accessed on:` |
| Portuguese (NBR native) | `Disponível em:` | `Acesso em:` |

For any other pinned language, use that language's natural equivalents; the structure never changes.
