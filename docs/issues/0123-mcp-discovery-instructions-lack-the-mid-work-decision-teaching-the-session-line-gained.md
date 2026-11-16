---
type: Issue
title: MCP discovery instructions lack the mid-work decision teaching the session line gained
description: Restore the parity ADR 0078 promises between the MCP instructions and the session-context line
status: closed
timestamp: 2026-11-15T16:24:30Z
---

## 0123. MCP discovery instructions lack the mid-work decision teaching the session line gained

### Scope

[ADR 0078](/adr/0078-an-mcp-front-serves-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-the-living-docs-mcp-subcommand.md) says the MCP server's discovery `instructions` give every MCP client what `skills/living-docs/hooks/session-context.sh` gives Claude Code. [Issue 0144](/issues/0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md) slice S3 added to the session line the teaching that a choice made mid-work goes into the active issue's Decision section, and that only a choice a future reader would pay to rediscover gets an ADR. `mcp/src/instructions.rs` › `instructions` does not carry it, so Codex and other MCP clients never receive the decision-capture teaching. Found by the review of that slice.

### Decision

- Bring `instructions()` to parity with the session line, phrased in the MCP tool vocabulary (`write` with operation `set`, the Decision section), with a test that asserts it.

### Acceptance

- The discovery instructions teach recording a mid-work choice in the active issue's Decision section and the materiality test for an ADR; a test in the mcp crate asserts both.
- `cargo test --workspace` and `make check` stay green.

### Outcome

- The MCP discovery instructions teach recording a material mid-work choice in the active issue's Decision through `write` with operation `set`, and the ADR materiality test. Proof: `mcp/tests/stdio_server/discovery.rs`.
