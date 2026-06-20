# Architecture Docs & Diagrams

Architecture documentation shows *how the system fits together* — the structure ADRs decide and the context index names, made visual. It is **living**: every diagram must match the code at all times, and a structural change updates its diagram in the same change (see `rules/maintenance-invariant.md`).

Diagrams are authored in **Mermaid** so they live in version control as text, diff cleanly in PRs, and render in most viewers — no binary image drift.

## Where it lives

- **Small project:** a single `docs/architecture.md` holding all diagrams.
- **Grown project:** an `docs/architecture/` directory with `index.md` + one file per view, organized by the same semantic-index contract as the context index (`rules/semantic-index.md`). Split when the single file passes ~200 lines or mixes unrelated views.

The architecture index (`index.md`, OKF reserved listing, no frontmatter) is reachable from the bundle-root `docs/index.md`. Each view file is a standalone **OKF concept** (`type: Architecture View`): frontmatter, then a single `#` H1, then `##` sections.

## The standard views

Cover the views the system actually has — don't invent diagrams for their own sake. Common ones:

| View | Mermaid type | Answers |
|---|---|---|
| **Context / high-level** | `flowchart` / `graph` | What are the major components and how do they connect to the outside world? |
| **Data model** | `erDiagram` | What entities exist and how do they relate? (schema, FKs, cardinality) |
| **Module layout** | `flowchart` | How is the code organized into modules and what depends on what? |
| **Process / data flow** | `flowchart` with direction | How does data move through a key operation (ingest, backfill, request)? |
| **Tool-calling / request flow** | `sequenceDiagram` | How does a request actually execute across actors over time? |
| **State** | `stateDiagram-v2` | What states does an entity move through? (lifecycles, retention) |

## Tool-calling / sequence diagrams (when applicable)

When the system's behaviour is best understood as a *conversation between actors over time* — an MCP/tool call, an agent invoking tools, a client→server→DB round trip — use a `sequenceDiagram` to make the flow concrete. Show the actors as participants and each call/return as a message. This is the clearest way to evidence "how it actually works" for tool-driven or multi-actor systems.

```mermaid
sequenceDiagram
    participant Client
    participant Server as MCP Server
    participant DB as SQLite
    Client->>Server: tool call (args)
    Server->>DB: query / write
    DB-->>Server: rows / result
    Server-->>Client: structured result
```

Include a sequence diagram only where it earns its place — a tool surface, a lifecycle with ordering, a non-obvious multi-step flow. Skip it for trivially linear calls.

## Rules

1. **Mermaid, in-repo, text.** No exported PNG/SVG that can silently drift from the code.
2. **No-drift.** A change to schema, data flow, module layout, or a component relationship updates the relevant diagram(s) in the *same* change. A structural PR with a stale diagram is incomplete.
3. **Use context-index vocabulary.** Name nodes and participants with the project's domain/module terms, not ad-hoc labels — diagrams and prose must agree.
4. **One view per diagram.** Don't cram the data model and the request flow into one graph. Split by concern; index each.
5. **Indexed.** Every view file is listed in the architecture index, which is listed in the top-level Docs index.

## Anti-patterns

- A diagram that contradicts the code — worse than no diagram, because it misleads with authority.
- Screenshot/exported-image diagrams that can't diff and rot immediately.
- A single mega-diagram trying to show everything at once.
- Diagrams added but never wired into the index (orphans).
