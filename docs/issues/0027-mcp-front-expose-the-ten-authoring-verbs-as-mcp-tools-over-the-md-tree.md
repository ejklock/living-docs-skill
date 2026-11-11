---
type: Issue
title: "MCP front: expose the ten authoring verbs as MCP tools over the .md tree"
description: A read-only MCP server crate in the workspace exposing search/show/list and, as they land, the graph verbs — agents consume the read-model through typed tools instead of parsing markdown.
status: Superseded
superseded_by: 0111
timestamp: 2026-08-05T20:33:40Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0111](0111-mcp-front-serve-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-living-docs-mcp.md). Run `living-docs read` for what is in force.

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## MCP front: expose the ten authoring verbs as MCP tools over the .md tree

**Closed 2026-09-22 — won't do.** MCP is not the right surface for living-docs. An agent already drives the CLI through its shell, and the CLI is the agent contract ([ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md)); an MCP front would duplicate that surface without a consumer that needs it. The body below is kept as the record of what was proposed.

Agents are first-class consumers of living docs, and today they consume them by shelling out to the CLI or parsing markdown. An MCP server front makes the bundle directly consumable by any MCP-capable agent, with typed tools and typed errors instead of exit codes and stdout parsing. Implements [ADR 0033](/adr/0033-new-consumers-are-fronts-in-the-workspace-never-new-repos-until-a-deploy-cadence-or-ownership-trigger-fires.md) (a new consumer is born as a workspace front). Precedent: Graphiti ships its MCP server from the same repository as its core.

**Rewritten 2026-09-22.** The original issue was written against the db-store read-model and the `search`/`show`/`list` verbs, plus the graph verbs of ADR 0031. [ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) deleted all of them: the `.md` tree in git is the only backend, and [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) locks the surface to ten verbs. Those premises are struck; the want — an agent consuming living-docs through typed tools — survives, so the scope below is restated against the verbs that exist.

### Scope

- New workspace member `mcp`: an MCP server (stdio transport first) depending on `living-docs-core` and `fs-store`, never on `cli`.
- Tools map one-to-one onto the ten verbs, reusing each verb's `core` entry point rather than shelling out: `new`, `set`, `supersede`, `index`, `check`, `fmt`, `read`, `guide`. `install`/`uninstall` stay CLI-only — they place files on a host, which is not a bundle operation.
- `check` returns the same structured report the CLI serializes (violations, advisories, `ok`), so an agent reads the gate without parsing text.
- Version site registered in `check-version.sh` in the same PR that creates the crate (ADR 0033 verification criterion).
- Out of scope: HTTP/SSE transport; any LLM-side logic (the server serves data, the consuming agent judges); any search or graph tool — there is no read-model to serve one from, and `read --topic` plus `grep` answer that question today (Constitution Amendment 1).

### Decision

Authoring tools ship in the first iteration rather than a read-only front, because the CLI's write verbs are already deterministic and gate-checked — the read-only restriction in the original issue existed to protect a db-store projection that no longer exists.

### Acceptance

- An MCP client lists the tools and gets a typed schema per verb; each schema's parameters match that verb's CLI contract (ADR 0060).
- `read` returns a record's frontmatter fields and body for a `TYPE/NNNN` reference, with the same cross-type collision error the CLI gives.
- `check` returns the structured report, and a bundle with a violation reports it as data, not as an exit code.
- The crate builds in the workspace, depends only on `living-docs-core` + `fs-store`, and its version site fails `check-version.sh` when out of sync (fitness).

### Plan

1. Crate skeleton + stdio MCP server + the read-side tools (`read`, `guide`, `check`, `index`).
2. The write-side tools (`new`, `set`, `supersede`, `fmt`), each gated by `check` exactly as the CLI is.
