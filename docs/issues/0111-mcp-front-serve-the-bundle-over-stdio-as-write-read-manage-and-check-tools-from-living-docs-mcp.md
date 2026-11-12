---
type: Issue
title: "MCP front: serve the bundle over stdio as write, read, manage and check tools from living-docs mcp"
description: "Implements ADR 0078: the living-docs mcp subcommand serves one bundle over stdio as four tools at parity with the CLI, teaches the contract through server instructions, and the session hook gains the read verbs."
status: open
supersedes: 0027
blocked_by: [0109]
timestamp: 2026-11-11T21:51:45Z
---

## 0111. MCP front: serve the bundle over stdio as write, read, manage and check tools from living-docs mcp

Implements [ADR 0078](/adr/0078-an-mcp-front-serves-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-the-living-docs-mcp-subcommand.md). Replaces [issue 0027](/issues/0027-mcp-front-expose-the-ten-authoring-verbs-as-mcp-tools-over-the-md-tree.md), closed as won't do because no consumer needed an MCP surface; the owner named that consumer on 2026-11-11. Blocked by [issue 0109](/issues/0109-regroup-the-cli-under-write-read-and-manage-with-check-top-level-and-move-every-surface-that-teaches-a-command.md), which gives the CLI the `write`/`read`/`manage` vocabulary the tools mirror.

### Scope

- New workspace member `mcp/` (crate `living-docs-mcp`), depending on `living-docs-core`, `fs-store` and `rmcp`, never on `cli`.
- `living-docs mcp --docs-dir <path>` in `cli/src/commands/mcp.rs`: starts the stdio server over one bundle.
- Four tools: `write` (`operation`: `new`, `set`, `supersede`), `read` (`records`, `guide`), `manage` (`index`, `fmt`), `check`; each operation's parameters match its CLI path, and record prose is a string parameter.
- `server/discover` returns `instructions` carrying the authoring contract and the read-before-deciding rule; the server also answers the 2025-11-25 `initialize` handshake.
- `skills/living-docs/hooks/session-context.sh` teaches the read verbs (`read records`, `--topic`, `--links-to`, `--impact`, `read guide`) next to the write verbs.
- `scripts/check-version.sh` registers the crate's version site; `README.md`, `npm/README.md` and `skills/living-docs/SKILL.md` show how to add the server to a client.
- Out of scope: the Streamable HTTP transport, the Tasks extension, records as resources and `subscriptions/listen`, and `install`/`uninstall`/`completions` as tools.

### Decision

- Full MCP front with write and read tools. Not taken: a read-only front; CLI only with better teaching (owner, 2026-11-11).
- The `session-context.sh` read-verb fix ships in this issue. Not taken: a separate issue delivered before the front, which the session recommended (owner, 2026-11-11).
- Transport: stdio now, Streamable HTTP later under its own ADR. Not taken: stdio only; HTTP only (owner, 2026-11-11).
- Distribution: the `living-docs mcp` subcommand of the same binary and npm package. Not taken: a separate `living-docs-mcp` binary (owner, 2026-11-11).
- Tools grouped as ADR 0076. Not taken: one tool per operation, which the session recommended for flat schemas (owner, 2026-11-11).
- Order: issue 0109 first, then this issue. Not taken: both in one release; the MCP front now over a flat CLI (owner, 2026-11-11).
- Bundle root: `--docs-dir` at launch. Not taken: client Roots with a fallback; a per-call `docs_dir` (owner, 2026-11-11).
- Tasks (SEP-2663) and `subscriptions/listen` are follow-ups with a trigger: Tasks enters with the HTTP transport, subscriptions when records become resources and a client needs to learn that a record changed mid-session. Not taken: either or both in the first slice (owner, 2026-11-11).
- The `--links-to`/`--impact` resolution into `Dependents`, which the CLI's `read records` and the MCP `read` tool both need, lives once in core's `effective` module and both fronts call it; S1 widens to `living-docs-core/src/commands/effective/dependents.rs` and `cli/src/commands/read.rs` for it. Not taken: committing S1 with the duplicate-code gate red and tracking it apart; excluding the block in the duplicate-code gate's config (owner, 2026-11-11).

### Acceptance

- An `rmcp` client over stdio lists exactly `write`, `read`, `manage` and `check` in a deterministic order, each with an input schema.
- For every operation, the MCP tool and the CLI command run on two copies of one fixture bundle leave identical files and return the same JSON.
- `write` `new` followed by `check` returns `ok: true`; a bundle with an unfilled slot returns that violation inside the `check` result, not as a tool error.
- A 2025-11-25 `initialize` and a 2026-07-28 `server/discover` both succeed, and the discover result carries `instructions`.
- `cargo tree -p living-docs-mcp` names no `cli` crate; `check-version.sh` fails on an out-of-sync crate version.
- `session-context.sh` output names `read records`, `--topic`, `--links-to` and `--impact`.
- `make check` and `se-gates check` pass.

### Plan

- S1: a library crate serves one bundle over stdio as the write, read (records), manage and check tools, dual-era (2025-11-25 initialize and 2026-07-28 server/discover with instructions), proved by an rmcp client test; the dependents resolution both fronts share moves into core | files: Cargo.toml, Cargo.lock, mcp/, scripts/check-version.sh, scripts/tests/check-version/run.sh, living-docs-core/src/commands/effective/dependents.rs, cli/src/commands/read.rs | after: none
- S2: living-docs mcp --docs-dir starts that server; read guide serves the skill corpus, moved where the mcp crate can reach it without depending on cli; a parity test finds each tool and its CLI path leave the same files and JSON | files: cli/src/args/, cli/src/commands/mcp.rs, cli/src/skill.rs, cli/src/commands/, mcp/, living-docs-core/src/ | after: S1, issue 0109
- S3: the session hook teaches the read verbs and README, npm README and SKILL.md show how to add the server to a client | files: skills/living-docs/hooks/session-context.sh, skills/living-docs/SKILL.md, README.md, npm/README.md | after: S2
