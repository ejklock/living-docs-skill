---
type: ADR
title: The MCP server publishes the embedded skills over the MCP skills extension with skill resources as fallback
description: The server declares io.modelcontextprotocol/skills, lists the four embedded skills and serves their files as skill:// resources for clients without the extension, raising rmcp to 3.12.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-27T02:00:09Z
---

# 0110. The MCP server publishes the embedded skills over the MCP skills extension with skill resources as fallback

## Context

ADR 0078 serves the authoring guide through the `read` tool's `guide` operation; a client must know to call it. The MCP skills extension, `io.modelcontextprotocol/skills` (SEP-2640, Final), lets a server publish skills that a host loads like its own: `skills/list` and `skills/get` return each skill's `SKILL.md` URI, its frontmatter and the URIs of its files, all `skill://<skill>/<file>` resources [1]. A client without the extension sees those as ordinary resources, readable through `resources/list` and `resources/read` [1]. Claude Code supports the extension in public beta from 2.1.331, except in `claude -p`, the Agent SDK, cloud sessions and, by default, Team and Enterprise accounts [2]. `rmcp` implements the server side from 3.9.0 and serves a skill directory with `SkillCatalog` behind the `skills-fs` feature from 3.10.0; 3.11.0 hardened path and name handling [3]. The workspace pins `rmcp` 3.9.

## Decision

We will serve the four embedded skills over the MCP skills extension, with their files as `skill://` resources, and raise `rmcp` to 3.12.

- The server declares the skills extension and the `resources` capability. `skills/list` returns `living-docs`, `living-docs-drift`, `okf-knowledge-format` and `research-artifacts` from the corpus embedded in the binary, the same corpus `manage install skills` places, so an npm or `install.sh` install serves them with no skills directory on disk.
- Every file of a skill is a `skill://<skill>/<path>` resource. A client without the extension reads the same files through `resources/read`; that is the fallback, as SEP-2640 defines it.
- The `read` tool's `guide` operation stays, for clients that call tools only.
- The Claude Code plugin of ADR 0108 keeps `skills/`, so an interactive Claude Code session lists each skill twice, once from the plugin and once from the server.

Rejected alternatives:

- **A prompt per skill as a second fallback.** SEP-2640 names resources as the fallback; a prompt adds a third surface for the same text.
- **Serve skills from a directory with `SkillCatalog::from_dir`.** The binary carries the corpus embedded; a directory exists only in a clone or the plugin cache.
- **Drop `skills/` from the plugin to avoid the duplicate.** MCP skills do not load in `claude -p`, the Agent SDK, cloud sessions or Team and Enterprise accounts by default.

## Consequences

**Easier / gained:**
- A host with the extension loads the authoring skills without `manage install skills`.
- One corpus feeds the CLI, the plugin and the server.

**Harder / accepted trade-offs:**
- `rmcp` moves three minor versions; the whole MCP suite reruns against it.
- Claude Code with the plugin shows each skill twice.

**Follow-ups:**
- Implemented as a slice of issue 0156.

## Verification

**Implementation impact:** `mcp/Cargo.toml`, `Cargo.lock`, the `mcp` crate's server handler, `mcp/tests/stdio_server/`.

**Verification criteria:**
- Over stdio, `skills/list` returns the four skills with frontmatter equal to the embedded `SKILL.md` files, and `skills/get` an unknown URI returns `-32602`.
- `resources/read` on each listed file URI returns the embedded bytes, whose sha256 equals the listed digest.

# References

[1] [SEP-2640 Skills Extension](https://github.com/modelcontextprotocol/modelcontextprotocol/blob/main/seps/2640-skills-extension.md)

[2] [Claude Code MCP: use skills from MCP servers](https://code.claude.com/docs/en/mcp)

[3] [modelcontextprotocol/rust-sdk CHANGELOG](https://github.com/modelcontextprotocol/rust-sdk)
