---
type: ADR
title: living-docs mcp starts without a bundle and resolves it per tool call
description: living-docs mcp starts whether or not the bundle exists; tools that need one return a tool error naming it, and write new creates it, amending ADR 0078 for the plugin of ADR 0108.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T01:47:55Z
---

# 0109. living-docs mcp starts without a bundle and resolves it per tool call

## Context

ADR 0078 serves one bundle per server entry from `living-docs mcp --docs-dir <path>`. `cli/src/commands/mcp.rs` refuses to start when that path is not a directory, printing `bundle root not found` and exiting 2. That fit a server registered per project with `claude mcp add`, where the bundle exists.

ADR 0108 ships the server in a Claude Code plugin. A plugin's MCP server starts in every project the user opens, so with the refusal, every project without a `docs/` bundle shows the `living-docs` server as failed at each session start. The CLI already treats a missing bundle per verb: `check` exits 2 naming the missing root, `read records` reports zero records, and `write new` creates the bundle with its first record and indexes.

## Decision

We will make `living-docs mcp` start whether or not the bundle directory exists, and resolve the bundle per tool call, as the CLI does per verb.

- A tool call that needs an existing bundle (`check`, `read`, `manage` except what the CLI already allows without one) returns a tool error naming the missing bundle root, the same message the CLI prints; the server keeps serving.
- `write new` creates the bundle, as the CLI does. Once it exists, every tool serves it in the same session.
- The bundle path stays fixed at launch and rooted as ADR 0073 roots the store; no tool takes another path.

Rejected alternatives:

- **Keep the refusal (ADR 0078 as is).** Every project without a bundle shows a failed server at each session start once the plugin is installed.
- **A plugin wrapper that skips the server without a bundle.** A stdio server that exits is shown as failed too, and the agent could not start a bundle in a new project over MCP.
- **Drop the MCP server from the plugin.** Brings back the per-project `claude mcp add` step the plugin exists to remove.

## Consequences

**Easier / gained:**
- The plugin's server is healthy in every project.
- An agent can start a bundle in a new project over MCP, with `write new`.

**Harder / accepted trade-offs:**
- A typo in `--docs-dir` no longer fails at start; it surfaces at the first tool call, as a tool error naming the path.

**Follow-ups:**
- Implemented as a slice of issue 0156.

## Verification

**Implementation impact:** `cli/src/commands/mcp.rs`, the `mcp` crate's bundle resolution, `mcp/tests/stdio_server.rs`.

**Verification criteria:**
- Over stdio with a missing bundle root, `tools/list` answers the four tools, `check` returns a tool error naming the root, and the process keeps serving.
- In the same session, `write new` creates the bundle and a following `read` lists the new record.
