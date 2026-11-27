---
type: Issue
title: The MCP check error on a missing bundle omits the CLI's prefix and hint
description: Over MCP, check on a missing bundle returns only bundle root not found with the path, while the CLI adds its verb prefix and a hint line.
status: open
timestamp: 2026-11-27T02:53:56Z
---

## 0159. The MCP check error on a missing bundle omits the CLI's prefix and hint

## Context

ADR 0109 says a tool that needs a bundle returns the same message the CLI prints. Over MCP, `check` on a missing bundle returns `bundle root not found: <path>` (`mcp/src/tools.rs` near line 249). The CLI prints `living-docs check: bundle root not found: <path>` and a hint line naming how to pass the docs directory (`cli/src/commands/check.rs` near line 66). The path and the core message match; the hint does not reach an MCP client. Found in review of issue 0156, slice S4.

## Decision

- One core function builds the missing-bundle message with its guidance (pass the docs directory, or create the bundle with `write new`); the CLI `check` and the MCP tools use it, the CLI keeping its `living-docs check:` prefix (session, 2026-11-27). Not taken: superseding ADR 0109 to say only the path is named.

## Acceptance

- The MCP `check` tool error on a missing bundle carries the same guidance the CLI hint gives, worded for a tool caller, or ADR 0109's wording is superseded to say only the path is named.

## Scope

In: `mcp/src/tools.rs` or a superseding ADR. Out: the CLI message.
