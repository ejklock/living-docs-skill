---
type: ADR
title: The MCP check tool runs as a task on a bundle above a measured record threshold
description: check returns an MCP task when the client declared the Tasks extension and the bundle reaches a benchmarked record count; everything else answers inline.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T02:00:29Z
---

# 0112. The MCP check tool runs as a task on a bundle above a measured record threshold

## Context

ADR 0078 rejected the Tasks extension because every operation is local and returns in milliseconds. In MCP 2026-07-28, Tasks is the extension `io.modelcontextprotocol/tasks` (SEP-2663): the server alone decides, per `tools/call`, whether to answer inline or return a task, and only when the client declared the extension; the client then polls `tasks/get` and may call `tasks/cancel` [1]. Claude Code supports it over stdio from 2.1.304 and polls every 2 s by default [2]. `rmcp` implements it with `CallToolResponse::Task` and `TaskManager` [3]. Measured on 2026-11-26, `living-docs check docs` over this repository's 281 records (1.8 MB) took 0.25 s in three runs; `check` grows with the bundle, so a large bundle can hold a call longer than a client's patience.

## Decision

We will run the `check` tool as a task when the client declared the Tasks extension and the bundle holds at least a threshold number of records; every other call, and `check` on a smaller bundle, answers inline.

- The threshold is a record count, fixed in code from a benchmark that grows a synthetic bundle until `check` takes 2 s on the CI runner, Claude Code's default poll interval. The benchmark and its result are committed with the threshold.
- A task's result is the same report the inline `check` returns. `tasks/cancel` stops the check and the task ends cancelled; no partial report is returned.
- A client without the extension always gets the inline answer.

Rejected alternatives:

- **Leave tasks out (ADR 0078 as is).** Recommended by the session for today's bundle sizes; the owner chose to prepare for large bundles.
- **Declare the extension and never create a task.** Announces support with no behaviour behind it.
- **A time budget instead of a record count.** The server must decide before running `check`, so it cannot know the time; the record count is known up front.

## Consequences

**Easier / gained:**
- A client gets a cancellable, pollable `check` on a large bundle instead of a blocked call.

**Harder / accepted trade-offs:**
- Polling and cancellation code for a case no current bundle reaches; the threshold must be re-measured when `check` gets faster or slower.

**Follow-ups:**
- Implemented as a slice of issue 0156.

## Verification

**Implementation impact:** the `mcp` crate's `check` tool and server capabilities, a benchmark under `scripts/bench/`, `mcp/tests/stdio_server/`.

**Verification criteria:**
- With the extension declared, `check` on a bundle at the threshold returns a task whose `tasks/get` ends with the same report as the inline call; one record below the threshold answers inline.
- Without the extension, `check` at the threshold answers inline.
- `tasks/cancel` on a running check ends the task cancelled.

# References

[1] [MCP Tasks extension 2026-07-28](https://modelcontextprotocol.github.io/ext-tasks/specification/2026-07-28/tasks.html)

[2] [Claude Code MCP](https://code.claude.com/docs/en/mcp)

[3] [modelcontextprotocol/rust-sdk](https://github.com/modelcontextprotocol/rust-sdk)
