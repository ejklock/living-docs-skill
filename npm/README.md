# living-docs

npm wrapper for the [`living-docs`](https://github.com/ejklock/living-docs-skill) CLI, the
deterministic layer of Living Docs authoring (`new`, `set`, `supersede`, `index`, `check`, `fmt`, `read`).

The package has no dependencies. It downloads the release binary for your platform from the
GitHub release matching the package version and verifies its sha256 before using it.

## Install

```bash
npm install -g living-docs
living-docs --version

npx living-docs check --plain docs
```

Supported platforms: macOS and Linux on x64 and arm64 (the four release targets
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`).

With `--ignore-scripts`, the binary is downloaded and verified on the first run instead.

## Environment

- `LIVING_DOCS_DOWNLOAD_BASE`: replaces `https://github.com/ejklock/living-docs-skill/releases/download/`
  (a mirror holding `v<version>/living-docs-<target>` and its `.sha256`).

## Fallback

If the download or the checksum fails, install the binary with the script instead:

```bash
curl -fsSL https://raw.githubusercontent.com/ejklock/living-docs-skill/main/install.sh | bash
```
