---
type: ADR
title: An MCP front serves the bundle over stdio as write, read, manage and check tools from the living-docs mcp subcommand
description: Add a workspace crate mcp, served by living-docs mcp over stdio, exposing write, read, manage and check tools grouped as ADR 0076 groups the CLI, at parity with the CLI, dual-era on the MCP protocol, with HTTP, Tasks and subscriptions left as follow-ups.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-11T21:51:26Z
---

# 0078. An MCP front serves the bundle over stdio as write, read, manage and check tools from the living-docs mcp subcommand

## Context

Issue 0027 proposed an MCP front and was closed as won't do on 2026-09-22: an agent drives the CLI through its shell, the CLI is the agent contract ([ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md)), and no consumer needed a second surface. [Issue 0111](/issues/0111-mcp-front-serve-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-living-docs-mcp.md) supersedes it and implements this decision.

On 2026-11-11 the owner reported that models do not use living-docs to its potential, in Claude Code, in Codex and in clients without a shell. Four failures were named: a client without a shell cannot reach the CLI at all; models do not find the verbs; models write record files by hand instead of `new` and `set`; models decide without reading what is already decided. The surfaces that teach the tool explain each one:

- `skills/living-docs/hooks/session-context.sh` emits one line that teaches only the write verbs (`new`, `set`, `supersede`, `fmt`, `index`) and names no read verb (`read`, `--topic`, `--links-to`, `--impact`, `guide`).
- That hook is a Claude Code `SessionStart` hook; Codex and other harnesses start a session without it and depend on the skill triggering.
- `skills/living-docs/SKILL.md` is a router that sends the model to `guide --list`, then `guide <topic>`: a verb is found only after two hops.
- `.githooks/pre-commit` runs `check` at commit; nothing acts inside the session.

A client without a shell is the consumer issue 0027 found missing, so its premise no longer holds.

The MCP specification revision 2026-07-28 is final. It removed the `initialize` handshake (every request carries its version in `_meta`), made `server/discover` mandatory with an optional `instructions` field of guidance for the model, deprecated Roots in favor of server configuration, moved Tasks to the `io.modelcontextprotocol/tasks` extension (SEP-2663), and replaced `resources/subscribe` with `subscriptions/listen`. A client that speaks only the earlier `initialize` revision fails against a server that speaks only 2026-07-28 [1][2]. The official Rust SDK, `rmcp` (3.9.0 on crates.io on 2026-11-11), implements 2026-07-28 and declares compatibility with 2025-11-25 [3].

## Decision

We will add an MCP front: a workspace crate `mcp` that depends on `living-docs-core` and `fs-store` only, served by the `living-docs mcp` subcommand of the same binary over the stdio transport. It exposes four tools grouped as [ADR 0076](/adr/0076-the-cli-groups-its-operations-under-write-read-and-manage-as-nested-subcommands-with-check-kept-top-level.md) groups the CLI:

| Tool | `operation` | CLI path |
|---|---|---|
| `write` | `new`, `set`, `supersede` | `write new\|set\|supersede` |
| `read` | `records`, `guide` | `read records\|guide` |
| `manage` | `index`, `fmt` | `manage index\|fmt` |
| `check` | none | `check` |

- **Parity with the CLI.** Each tool calls the same `core` entry point as its CLI path, so an MCP call and the equivalent CLI command leave the same bytes on disk and return the same JSON shape. Record prose arrives as a string parameter, which is how [ADR 0066](/adr/0066-record-prose-enters-through-a-file-or-stdin-new-body-and-set-from-replace-argv-for-body-and-field-values.md) keeps it out of argv in a protocol with no argv.
- **The gate reports as data.** `check` returns its report (`ok`, violations, advisories) as a result, not as a tool error; `check` at commit and in CI stays the only enforcement.
- **The bundle comes from the launch.** `living-docs mcp --docs-dir <path>` fixes one bundle per server entry, rooted as [ADR 0073](/adr/0073-the-filesystem-store-is-rooted-at-the-bundle-and-refuses-any-read-write-or-rename-that-resolves-outside-it.md) roots the store; no tool takes a path to another bundle.
- **The server teaches at discovery.** The `instructions` field carries the authoring contract and the read-before-deciding rule in the ADR 0076 vocabulary, so every MCP client receives what `session-context.sh` gives Claude Code; that hook gains the read verbs in the same change.
- **Dual-era protocol.** The server serves revision 2026-07-28 and answers the 2025-11-25 `initialize` handshake, so clients that have not moved to the stateless revision still connect.
- **Order.** The front ships after [issue 0109](/issues/0109-regroup-the-cli-under-write-read-and-manage-with-check-top-level-and-move-every-surface-that-teaches-a-command.md) implements ADR 0076, so the CLI, the tools, the instructions and the hook share one vocabulary.
- **Out of the front.** `install`, `uninstall` and `completions` place files on a host and stay CLI-only. The Streamable HTTP transport, the Tasks extension and `subscriptions/listen` are not part of this decision.

Rejected alternatives:

- **Keep the CLI as the only surface and fix the teaching** (the existing pattern). Rejected: it leaves the client without a shell with no path, and a hook reaches only the harness that runs it.
- **A read-only MCP front** (`read`, `guide`, `check`). Rejected: a client without a shell can then consult the bundle but never author it.
- **One tool per operation** (`new`, `set`, `supersede`, `index`, `fmt`, `check`, `read`, `guide`). Recommended in the interview for its flat schemas and rejected by the owner, who chose one vocabulary across the CLI and the MCP front; the cost of a `oneOf` schema per tool is accepted and measured.
- **A separate `living-docs-mcp` binary.** Rejected: a second release asset, npm bin and version site for no consumer that needs them apart.
- **Client Roots or a per-call `docs_dir` to find the bundle.** Rejected: revision 2026-07-28 deprecates Roots and points to server configuration, and a per-call path puts the bundle root in the public schema.
- **Ship HTTP, Tasks or subscriptions in the first slice.** Rejected: every operation is local and returns in milliseconds, so a task would never be created, and records are not yet exposed as resources to subscribe to.

## Consequences

**Easier / gained:**
- A client without a shell (Claude Desktop) authors and reads a bundle.
- Every MCP client sees the four tools with typed schemas and the contract in `instructions`, without depending on a skill trigger or a harness hook.
- `check` violations reach the model as structured data.

**Harder / accepted trade-offs:**
- The tool schemas become a second public contract next to the CLI's JSON shapes: a parameter is added, never renamed or removed.
- The binary gains an async runtime (`tokio`) and `rmcp`; build time and binary size grow.
- A grouped tool carries a `oneOf` schema over its operations, which a model may call wrongly more often than a flat one.
- The MCP front does not stop a model with file tools from writing a record by hand; `check` remains what catches it.

**Follow-ups:**
- An issue for the Streamable HTTP transport with its own ADR, opened when the owner needs a web client (claude.ai, ChatGPT); the Tasks extension enters with it, for the remote operations that run long.
- An issue to expose records as resources with `subscriptions/listen`, opened when a client needs to learn mid-session that a record it read changed or was superseded.

## Verification

**Implementation impact:** `Cargo.toml` (workspace members), a new `mcp/` crate, `cli/src/args.rs` and `cli/src/commands/mcp.rs` (the subcommand), `scripts/check-version.sh` (the crate's version site), `skills/living-docs/hooks/session-context.sh`, `skills/living-docs/SKILL.md`, `README.md`, `npm/README.md`.

**Verification criteria:**
- An `rmcp` client test over stdio lists exactly `write`, `read`, `manage` and `check`, each with an input schema, in a deterministic order.
- A parity test runs each operation through the MCP tool and through the CLI on two copies of one fixture bundle and finds the same files byte for byte and the same JSON result.
- `write` with `operation: new` followed by `check` returns `ok: true` through MCP (the ADR 0001 fitness function over the new front); a bundle with an unfilled slot returns the violation in the `check` result, not as a tool error.
- A 2025-11-25 `initialize` request and a 2026-07-28 `server/discover` request both succeed, and the discover result carries `instructions`.
- `cargo tree -p living-docs-mcp` shows no dependency on the `cli` crate; `check-version.sh` fails when the crate's version is out of sync.

# References

[1] [MCP specification 2026-07-28, key changes](https://modelcontextprotocol.io/specification/2026-07-28/changelog)

[2] [MCP 2026-07-28, versioning and compatibility](https://modelcontextprotocol.io/specification/2026-07-28/basic/lifecycle) and [server/discover](https://modelcontextprotocol.io/specification/2026-07-28/server/discover)

[3] [modelcontextprotocol/rust-sdk README](https://github.com/modelcontextprotocol/rust-sdk)
