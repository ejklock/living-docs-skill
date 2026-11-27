---
type: ADR
title: A Claude Code plugin and marketplace at the repository root ship the skills, the MCP server and the session hook
description: The repository root is a Claude Code marketplace listing one living-docs plugin that ships skills/, the MCP server through npx and a SessionStart hook, version-locked to VERSION, as an exception to ADR 0028.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T01:08:05Z
---

# 0108. A Claude Code plugin and marketplace at the repository root ship the skills, the MCP server and the session hook

## Context

Claude Code reaches living-docs in three steps today: install the binary (`install.sh` or `npm`), place the skills and hooks with `living-docs manage install skills` and `manage install hooks`, and register the MCP server from ADR 0078 by hand with `claude mcp add`. The owner wants one step in Claude Code: `/plugin install`.

A Claude Code plugin is a directory with an optional `.claude-plugin/plugin.json`. Claude Code discovers `skills/<name>/SKILL.md`, `hooks/hooks.json` and `.mcp.json` at its root [1]. Users install plugins only from a marketplace, which is a `.claude-plugin/marketplace.json` listing plugins by `name` and `source` [2]. Claude Code copies the plugin source into `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/`. It exposes that path as `${CLAUDE_PLUGIN_ROOT}` and the user's project as `${CLAUDE_PROJECT_DIR}` [1][3]. The documentation does not state the working directory of a plugin's stdio MCP server [3]. A pinned `version` in `plugin.json` holds every user on that version until the string changes; without it, every commit is an update [4]. Plugin hooks load with the session and fire in every project the user opens, not only in projects that opted in [5].

This collides with ADR 0028. That ADR says *"Nothing installs by copying out of a checked-out tree"*, so skills reach a harness only from the corpus embedded in the release binary. A plugin is a copy of a tree by construction. The skills must sit inside the plugin root, so they cannot come from the binary.

## Decision

We will make the repository root both a Claude Code marketplace and the `living-docs` plugin it lists. The plugin ships the skills, the MCP server and the session hook. This is an exception to ADR 0028 for this one channel; every other harness keeps `manage install skills`.

- **One repository, two roles.** `.claude-plugin/marketplace.json` lists one plugin, `living-docs`, whose `source` is the repository root. `.claude-plugin/plugin.json` names the same plugin. Users install it with `claude plugin marketplace add ejklock/living-docs-skill` and then `claude plugin install living-docs@living-docs`.
- **Skills from `skills/`.** Claude Code discovers the four skills in place. Nothing is copied or generated into a second directory.
- **Version lockstep.** `plugin.json` declares `version` equal to `VERSION`, and `scripts/check-version.sh` gates it with every `SKILL.md`. A user receives an update only at a release, and the skills at a release tag are the corpus that release's binary embeds.
- **The MCP server through npm.** `plugin.json` declares it inline under `mcpServers`, so the repository's own untracked `.mcp.json` stays a developer's local file. It runs `npx -y @ejklock/living-docs@<VERSION> mcp --docs-dir ${CLAUDE_PROJECT_DIR}/docs`. The binary comes from ADR 0107's verified first-run download, at the plugin's own version. The docs path is absolute because the server's working directory is undocumented.
- **One hook: `SessionStart`.** `plugin.json` declares it inline under `hooks`, with no `hooks/` directory at the root. It runs `skills/living-docs/hooks/session-context.sh` from `${CLAUDE_PLUGIN_ROOT}`. It prints the session line only when the project has a `docs/` bundle, and it never changes git configuration. The `PostToolUse` decision prompt stays opt-in per project (ADR 0089). The `pre-commit` doc-gate stays with `manage install hooks`, because a plugin cannot place a git hook.

Rejected alternatives:

- **A plugin with only the MCP server, in a `plugin/` subdirectory.** It keeps ADR 0028 intact, but installation stays two steps (plugin, then `manage install skills`), which is the step the owner wants removed.
- **Copy the skills into a plugin subdirectory.** It needs a second copy of the corpus and a drift gate for it, only to keep a principle this decision already bends.
- **Ship the `PostToolUse` decision prompt too.** A plugin hook fires in every project, so the opt-in nudge ADR 0089 decided would become always-on.
- **Leave `version` out and track commits.** Every push to `main` would reach users before a release exists, and the npm version the MCP server pins would not match.
- **Keep the current three steps.** Rejected by the owner.

## Consequences

**Easier / gained:**
- One step in Claude Code installs the skills, the MCP server and the session line.
- No second copy of the corpus: the plugin reads the same `skills/` the binary embeds.
- The plugin, the npm package and the binary share one version.

**Harder / accepted trade-offs:**
- ADR 0028 now has an exception. The Claude Code plugin installs from a copy of the tree.
- The marketplace clone carries the whole repository (sources, docs, tests), not only the plugin's files.
- The MCP server needs Node.js and network access on its first run, as ADR 0107 accepts for npm.
- A user with both the plugin and `manage install hooks` in a project gets the session line twice.
- The exact `"source": "./"` root-plugin layout is recognized but not shown in the documentation; `claude plugin validate .` is its proof.

**Follow-ups:**
- An issue that implements this decision.
- At acceptance, annotate ADR 0028's Decision with a pointer to this ADR.

## Verification

**Implementation impact:** `.claude-plugin/marketplace.json`, `.claude-plugin/plugin.json`, `skills/living-docs/hooks/session-context.sh`, `scripts/check-version.sh`, `README.md`.

**Verification criteria:**
- `claude plugin validate .` passes at the repository root in CI.
- `scripts/check-version.sh` fails when `plugin.json` `version` or the npm version in its `mcpServers` entry differs from `VERSION`.
- A test runs the plugin's `SessionStart` command in a repository without `docs/` and with a `.githooks/` directory: it prints nothing and leaves `core.hooksPath` unset.
- A test runs the same command in a repository with `docs/`: it prints the session line.

# References

[1] [Claude Code plugins reference: manifest, standard layout and environment variables](https://code.claude.com/docs/en/plugins/manifest-reference.md)

[2] [Create and distribute a plugin marketplace](https://code.claude.com/docs/en/plugins/create-marketplace.md)

[3] [Claude Code MCP: plugin-provided MCP servers](https://code.claude.com/docs/en/mcp.md)

[4] [Plugin loading and versioning](https://code.claude.com/docs/en/plugins/loading.md)

[5] [Plugin components: hooks](https://code.claude.com/docs/en/plugins/components.md)
