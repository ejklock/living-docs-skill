# living-docs

npm wrapper, published as `@ejklock/living-docs` (the installed command stays `living-docs`),
for the [`living-docs`](https://github.com/ejklock/living-docs-skill) CLI, the deterministic
layer of Living Docs authoring (`write`, `read`, `manage`, `check` and `mcp`).

The package has no dependencies and runs no install script. The first run downloads the
release binary for your platform from the GitHub release matching the package version,
verifies its sha256 and starts it; later runs start the saved binary with no network request.

## Install

```bash
npm install -g @ejklock/living-docs
living-docs --version

npx @ejklock/living-docs check --plain docs
```

Supported platforms: macOS and Linux on x64 and arm64 (the four release targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`).

The binary is saved in the package's `vendor/` directory when the user running the command can
write to it, otherwise (for example a global install owned by root) in the user cache at
`$XDG_CACHE_HOME/living-docs/<version>/living-docs`, or `~/.cache/living-docs/<version>/living-docs`
when `XDG_CACHE_HOME` is unset or not an absolute path. The first run needs network access to GitHub.

## Use it from an MCP client

`living-docs mcp --docs-dir docs` serves the bundle over stdio as the write, read, manage
and check tools. Run it from the project root, so `docs` resolves. Add it to Claude Code:

```bash
claude mcp add living-docs -- living-docs mcp --docs-dir docs
```

A client configured by an `mcpServers` JSON object (Claude Desktop, Cursor) takes:

```json
{
  "mcpServers": {
    "living-docs": {
      "command": "living-docs",
      "args": ["mcp", "--docs-dir", "docs"]
    }
  }
}
```

Without a global install, run the package through npx. For Claude Code:

```bash
claude mcp add living-docs -- npx -y @ejklock/living-docs mcp --docs-dir docs
```

For an `mcpServers` JSON object, `command` holds the program alone and `args` the rest:

```json
{
  "mcpServers": {
    "living-docs": {
      "command": "npx",
      "args": ["-y", "@ejklock/living-docs", "mcp", "--docs-dir", "docs"]
    }
  }
}
```

A client that does not start the server in the project root needs an absolute `--docs-dir`,
in either form:

```text
"args": ["mcp", "--docs-dir", "/path/to/project/docs"]
"args": ["-y", "@ejklock/living-docs", "mcp", "--docs-dir", "/path/to/project/docs"]
```

## Environment

- `LIVING_DOCS_DOWNLOAD_BASE`: replaces `https://github.com/ejklock/living-docs-skill/releases/download/`
  (a mirror holding `v<version>/living-docs-<target>` and its `.sha256`).
- `LIVING_DOCS_PLATFORM`: replaces the detected `<platform>-<arch>` key, such as `linux-x64`.
- `XDG_CACHE_HOME`: the cache root used when the package directory is read-only.

## Fallback

If the download or the checksum fails, install the binary with the script instead:

```bash
curl -fsSL https://raw.githubusercontent.com/ejklock/living-docs-skill/main/install.sh | bash
```
