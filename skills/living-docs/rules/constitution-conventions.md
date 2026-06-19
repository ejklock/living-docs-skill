# Constitution Conventions

The Product Constitution is the **foundational source of truth** for a project: what the product is, what it is not, the data model it is built on, and the invariants that hold in all circumstances. Every other document in the doc trail sits under it and must be consistent with it.

The doc trail flows: **constitution → PRD → ADR + BDR → issues → code**.

## Format

The constitution is an **OKF concept** (`type: Constitution`) — see the `okf-knowledge-format` skill. `status` (`Draft` | `Ratified` | `Amended`) lives in the frontmatter, not a body line. See `templates/constitution.md`. Core sections:

- **Product** — the core value and the audience in one or two sentences. The north star.
- **Scope boundaries** — what is in scope, what is explicitly out, and what defers to which phase.
- **Data model / schema foundation** — the core entities and relationships as a Mermaid diagram, with prose describing invariants.
- **Non-negotiables** — constraints that hold regardless of feature, phase, or implementation. Each must be falsifiable.
- **Amendment log** — dated amendments appended below the original content; sections above are immutable once ratified.

## Rules

1. **One per project.** The constitution lives at `docs/constitution.md`. There is no NNNN prefix and it is not listed as a concept in any `index.md` — it is singular, the bundle's root of trace.
2. **PRDs sit under the constitution; they never replace it.** A PRD specifies a feature or capability within the scope the constitution defines. If a PRD requires expanding that scope, the constitution must be amended first.
3. **Only foundational scope or schema shifts amend the constitution.** Adding a feature does not amend the constitution. Changing what the product fundamentally is, who it is for, or what its core data model looks like does.
4. **Append-only once ratified.** After the constitution is ratified, changes are recorded as dated Amendment sections at the bottom (`## Amendment N — YYYY-MM-DD: <summary>`). The original sections above are never silently edited.
5. **Diagrams are Mermaid only.** No ASCII art, no image attachments.
6. **Non-negotiables are falsifiable.** "Be secure" is not a non-negotiable. "All user data at rest is encrypted with AES-256" is.
7. **The constitution is the root of the trace.** When reviewing any PRD, ADR, BDR, or issue, the chain should resolve back to the constitution. Work that cannot be traced to the constitution is out of scope.
8. **The doc language is a non-negotiable.** If the user has declared a documentation language (default English otherwise), pin it here as a non-negotiable line so it survives across sessions — see `rules/doc-language.md`.

## Anti-patterns

- Writing a constitution per feature instead of per project. One project, one constitution.
- A PRD that implicitly changes scope without amending the constitution first — the constitution and PRD are then contradictory.
- Treating the constitution as a living editable document after ratification. Silent edits break the paper trail; amend instead.
- Non-negotiables that are aspirational rather than falsifiable — they provide no enforcement anchor.
- An empty Scope Boundaries section. If the out-of-scope items are not named, scope is undefined and PRDs will drift.
