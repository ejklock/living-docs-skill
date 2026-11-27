---
type: Issue
title: Ship living-docs as a Claude Code plugin from a marketplace at the repository root
description: "Implement ADR 0108: marketplace and plugin manifests at the root, MCP through npx, a SessionStart hook that never touches git config, and a version gate."
status: open
timestamp: 2026-11-27T01:08:34Z
---

## 0156. Ship living-docs as a Claude Code plugin from a marketplace at the repository root

## Context

ADR 0108 decides that the repository root is a Claude Code marketplace and the `living-docs` plugin it lists, shipping `skills/`, the MCP server from ADR 0078 and a `SessionStart` hook. This issue carries the implementation.

## Decision

- Plugin scope: full plugin at the repository root, with skills, MCP and the session hook (owner, 2026-11-26). Not taken: a plugin with only the MCP server in `plugin/`, which keeps ADR 0028 intact; keeping the current three steps.
- Hooks: `SessionStart` only (owner, 2026-11-26). Not taken: `SessionStart` with the `PostToolUse` decision prompt, which would make ADR 0089's opt-in nudge always-on; no hook at all.
- The marketplace lives in this repository too (owner, 2026-11-26).
- `mcpServers` and `hooks` are inline in `plugin.json`: the root `.mcp.json` is a developer's untracked local file, ignored globally. Not taken: a tracked root `.mcp.json` and a root `hooks/hooks.json`.
- Plugin mode of `session-context.sh` is the environment variable `LIVING_DOCS_PLUGIN=1`, set by the plugin's hook command: the script then prints nothing unless `$CLAUDE_PROJECT_DIR/${LIVING_DOCS_BUNDLE:-docs}` is a directory, never runs `git config`, and, with no binary on `PATH`, names the npm and `install.sh` installs instead of `make build`. Not taken: a second script for the plugin, which duplicates the session line.
- A project without a bundle: `living-docs mcp` starts anyway and resolves the bundle per tool call, recorded as ADR 0109 (owner, 2026-11-26). Not taken: keeping the start refusal, so the plugin's server shows as failed in every project without `docs/`; dropping the MCP server from the plugin.
- The plugin-mode project marker is `$BUNDLE/index.md` holding an `okf_version:` frontmatter line, which `write new` and `manage index` write on every bundle, not the bare `docs/` directory: unrelated projects (MkDocs, Docusaurus) have a `docs/` folder and would receive the authoring line. Found in review of S1. Not taken: a type index such as `adr/index.md`, which a bundle with only issues lacks.
- MCP skills, events and tasks enter this issue as further slices, not separate issues (owner, 2026-11-26). Not taken: one issue and ADR per item after the plugin ships. Each still gets its own ADR amending ADR 0078 before code, after research on the current MCP specification and the `rmcp` version in use.
- Skills over MCP use the protocol's native skills support where the client has it, with a prompt or resource fallback for clients without it (owner, 2026-11-26). Not taken: prompts only; resources only.
- MCP skills: the native skills extension (`io.modelcontextprotocol/skills`, SEP-2640) over the embedded corpus, with `rmcp` raised from 3.9 to 3.12; clients without it read the same `skill://` files as ordinary resources, as the SEP defines (owner, 2026-11-26). Not taken: a prompt per skill as a second fallback.
- The plugin keeps `skills/` although Claude Code then lists each skill twice, once from the plugin and once from the MCP server (owner, 2026-11-26): MCP skills do not load in `claude -p`, the Agent SDK, cloud sessions, or Team and Enterprise accounts by default. Not taken: dropping `skills/` from the plugin.
- MCP events: records become resources, and the server emits `list_changed` and `resources/updated` through `subscriptions/listen` both for its own writes and for edits a file watcher sees outside the server (owner, 2026-11-26). Not taken: events for the server's own writes only, which needs no new dependency.
- MCP tasks: `check` becomes a task over the Tasks extension (SEP-2663) when the bundle is larger than a measured threshold and the client declared the extension (owner, 2026-11-26). Not taken: leaving tasks out (the session's recommendation, since every call takes milliseconds); declaring the extension without ever creating a task.
- The plugin-mode marker premise was wrong: the CLI writes `okf_version` only on a root it creates (`living-docs-core/src/commands/index/root.rs:65`), found in review of S5. `write new` and `manage index` now add it to an existing root that lacks it, recorded as ADR 0113 (owner, 2026-11-26). Not taken: a second marker in the hook only (the session's recommendation); `check` reporting the missing key; accepting the silence. `manage fmt` stays out because reserved files are outside its scope (`living-docs-core/src/commands/fmt.rs:154`).
- Record resource URIs are `record://` plus the reference `write set` already accepts (`record://adr/0108`, `record://view/crates`, `record://constitution`); a domain record, which has no number, uses its bundle-relative path (`record://context/billing/invoice.md`). Change events coalesce over 200 ms. Not taken: a `living-docs://` scheme, longer to type with no gain; a per-event emit, which floods a client on `git checkout`.
- The task threshold of ADR 0112 is measured on the development host, not the CI runner, and the host's CPU and the measured curve are committed with the number (session, 2026-11-27): CI runs heavy jobs only on manual dispatch, and a benchmark job is its own scope. Not taken: a manual-dispatch CI benchmark job now. Re-measuring on the CI runner is a follow-up issue.
- Cancelling a `check` task stops the computation, not only the answer, as ADR 0112 says (owner, 2026-11-27): core's `check` gains a cooperative cancellation point checked between records, and the MCP task passes its cancel signal to it. Not taken: ending the task cancelled while the computation runs to its end and its result is discarded. Delivered as slice S8b after S8, so the slice in flight keeps its scope.
- Slices run as a chain in the main checkout: the plugin's hook command is proved through the real plugin mode, and the version gate reads the real `plugin.json`.

## Acceptance

- `.claude-plugin/marketplace.json` and `.claude-plugin/plugin.json` exist, name the plugin `living-docs`, and `claude plugin validate .` passes at the repository root in CI.
- `plugin.json` `version` and the npm version pinned in its `mcpServers` entry equal `VERSION`; `scripts/check-version.sh` fails on a mismatch, proved by a case in `scripts/tests/check-version/run.sh`.
- The plugin's `mcpServers` entry starts `npx -y @ejklock/living-docs@<VERSION> mcp --docs-dir ${CLAUDE_PROJECT_DIR}/docs`.
- The plugin's `SessionStart` hook prints nothing in a project without `docs/` and leaves `core.hooksPath` unset in a project with `.githooks/`; in a project with `docs/` it prints the session line. Both are proved by a script test.
- Over stdio with a missing bundle root, `living-docs mcp` lists its four tools, `check` returns a tool error naming the root, and `write new` creates the bundle (ADR 0109).
- `manage install hooks` keeps arming `core.hooksPath`, proved by its existing tests.
- `README.md` documents the two install commands next to the existing MCP setup.
- ADR 0108 is set to Accepted and ADR 0028's Decision carries a pointer to it.

## Scope

In: the two manifests with inline `mcpServers` and `hooks`, the plugin mode of `session-context.sh`, the version gate, the CI validate step, the README.

Out: the `PostToolUse` decision prompt and the `pre-commit` doc-gate in the plugin; plugins for other harnesses; submission to Anthropic's official marketplace.

## Plan

- S1: `session-context.sh` plugin mode stays silent without a bundle, never touches git config, and prints the session line with one | files: `skills/living-docs/hooks/session-context.sh`, `cli/tests/session_context_line.rs` | after: none
- S2: the repository root is a valid marketplace listing the `living-docs` plugin with inline `mcpServers` and `SessionStart` hook, validated in CI and documented | files: `.claude-plugin/marketplace.json`, `.claude-plugin/plugin.json`, `.github/workflows/ci.yml`, `README.md` | after: S1, S4, S5, S9
- S3: `check-version.sh` fails when `plugin.json` `version` or its `mcpServers` npm pin differs from `VERSION` | files: `scripts/check-version.sh`, `scripts/tests/check-version/run.sh`, `.github/workflows/release.yml` | after: S2
- S4: `living-docs mcp` starts without a bundle; tools needing one return a tool error naming it, and `write new` creates it | files: `cli/src/commands/mcp.rs`, the `mcp` crate's bundle resolution, `mcp/tests/stdio_server.rs` | after: none
- S5: plugin mode recognizes a living-docs project by `$BUNDLE/index.md` with `okf_version:`, staying silent in a project whose `docs/` is not a bundle | files: `skills/living-docs/hooks/session-context.sh`, `cli/tests/session_context_line.rs` | after: S1
- S6: `rmcp` 3.12; the server lists the four embedded skills over the skills extension and serves their files as `skill://` resources (ADR 0110) | files: `mcp/Cargo.toml`, `Cargo.lock`, the `mcp` crate's handler, `mcp/tests/stdio_server/` | after: S4
- S7: records are resources; `subscriptions/listen` delivers `list_changed` and `updated` from server writes and a coalescing file watcher (ADR 0111) | files: the `mcp` crate, `mcp/Cargo.toml`, `mcp/tests/stdio_server/` | after: S6
- S8: `check` runs as a task above a benchmarked record threshold when the client declared the Tasks extension (ADR 0112) | files: the `mcp` crate's `check` tool, `scripts/bench/`, `mcp/tests/stdio_server/` | after: S6
- S9: `write new` and `manage index` add `okf_version` to an existing root that lacks it (ADR 0113); the S5 no-frontmatter fixture gains an `okf_version:` line outside frontmatter | files: `living-docs-core/src/commands/index/root.rs`, its tests, `cli/tests/session_context_plugin_mode.rs` | after: S5, S4
- S10: the CLI and MCP parity harness gains a missing-bundle fixture with rows for `read records`, `check`, `manage index`, `manage fmt` and `write new`, so a drift on either side fails a test (review of S4) | files: `cli/tests/mcp_parity.rs`, `cli/tests/common/parity.rs` | after: S4
- S9b: repair of the S9 review's normal findings: a root whose first line is a `---` rule and whose block does not parse as YAML counts as no frontmatter (a valid block goes above it); `fresh_bundle.rs` asserts the exact migrated empty root; nits: the `index_crlf.rs` docblock and `root_is_listed` matching the root path | files: `living-docs-core/src/commands/index/root.rs`, `root_tests.rs`, `cli/tests/fresh_bundle.rs`, `cli/tests/index_crlf.rs`, `cli/tests/root_index_marker.rs` | after: S9, S6
- S11: a root `index.md` that starts with a UTF-8 BOM is marked after the BOM, and the session hook skips a leading BOM, so a BOM root is neither split nor marked twice (S9 review) | files: `living-docs-core/src/commands/index/root.rs`, `skills/living-docs/hooks/session-context.sh`, their tests | after: S9b
- S6b: repair of the S6 review's normal findings: skill frontmatter parsed once in core (`living_docs_core::guide`), dropping `serde_yaml` from the `mcp` crate; `skills/get` rejects an installable non-entry file of another skill; nits: the duplicate dev-dependency, `text/x-shellscript` for `drift.sh`, error codes asserted | files: `living-docs-core/src/guide.rs`, `mcp/src/skills.rs`, `mcp/Cargo.toml`, `Cargo.lock`, `mcp/tests/stdio_server/skills.rs` | after: S6, S9b
- S7b: repair of the S7 review's normal findings: one public in-force record listing in core used by the read tool and the record resources; server writes coalesced with watcher events over the 200 ms window; a test that the watcher threads end with the last listener; nits: an `index.md` with `type:` frontmatter in the fixtures, the first scan off the lock, percent-encoded path URIs | files: `living-docs-core/src/commands/effective.rs`, `mcp/src/records.rs`, `mcp/src/events.rs`, `mcp/tests/stdio_server/` | after: S7
- S10b: repair of the S10 review: a missing-bundle row also proves the docs directory itself is not created; nits: the untouched failure message, a `shows=` on the write new row | files: `cli/tests/common/parity.rs`, `cli/tests/mcp_parity.rs` | after: S10
- S8b: `tasks/cancel` on a running `check` task stops the computation through a cooperative cancellation point in core's `check`, checked between records; the CLI `check` is unchanged | files: `living-docs-core/src/check*`, `mcp/src/tasks.rs`, their tests | after: S8
- S11b: repair of the S11 review: tests pin that only one leading BOM is stripped (a double-BOM root in core, a BOM on line 2 in the hook), and the hook's awk uses the POSIX octal escape `\357\273\277` instead of `\xef\xbb\xbf` | files: `living-docs-core/src/commands/index/root_marker_tests.rs`, `skills/living-docs/hooks/session-context.sh`, `cli/tests/session_context_plugin_mode.rs` | after: S11
