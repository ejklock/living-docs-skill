---
type: ADR
title: The npm package installs with no install script and the shim downloads the verified binary on first run
description: The npm wrapper drops its postinstall; the shim downloads and verifies the release binary on first run, into the package or the user cache.
owner: Evaldo Klock
status: Accepted
supersedes: 0077
timestamp: 2026-11-24T15:01:18Z
---

# 0107. The npm package installs with no install script and the shim downloads the verified binary on first run

## Context

[ADR 0077](0077-the-npm-package-is-published-under-the-scoped-name-ejklock-living-docs-because-the-unscoped-name-belongs-to-another-author.md) keeps the shape [ADR 0067](0067-an-npm-package-wraps-the-released-binary-as-a-second-distribution-channel-so-npx-living-docs-resolves.md) gave the npm wrapper: a `postinstall` script downloads the release asset for the host and verifies its sha256, and the shim repeats that download on first run when the install ran with `--ignore-scripts`. Socket rates the package's supply chain security at 65 of 100 (socket.dev page for `@ejklock/living-docs` 0.31.0, seen by the owner on 2026-11-24). Install scripts with network access are the pattern supply-chain scanners weigh most, because they run code on every `npm install`, before anyone calls the tool; which alerts Socket lists was not read (the page refuses automated reads), so the weighting is an inference. The lazy download in the shim already does everything the `postinstall` does, so the install script adds only an earlier download.

## Decision

We will publish `@ejklock/living-docs` with no install script, and the shim downloads and verifies the binary on first run; everything else ADR 0077 decided stays.

- **No install script.** `npm/package.json` declares no `preinstall`, `install` or `postinstall`; `npm/scripts/postinstall.js` is removed and `files` no longer lists `scripts`. `npm install` runs no package code and makes no network request.
- **First run.** `bin/living-docs.js` downloads `living-docs-<target>` and its `.sha256` from the GitHub release matching the package version, verifies the checksum, writes the binary atomically and runs it, as the `--ignore-scripts` path does today. Every later run starts the cached binary with no network request. A failure removes any partial file, exits non-zero and names the `install.sh` fallback; nothing is written to stdout, so an MCP client reading stdio sees no stray bytes.
- **Where the binary goes.** Inside the package (`vendor/living-docs`) when that directory is writable; otherwise, as after a global install by root, under the user's cache directory (`$XDG_CACHE_HOME`, else `~/.cache`) at `living-docs/<version>/living-docs`. The version in the path keeps an upgrade from running an older cached binary.
- **Environment overrides stay.** `LIVING_DOCS_DOWNLOAD_BASE` (a mirror) and `LIVING_DOCS_PLATFORM` (the target key) remain: the fixture tests inject the stub server and the platform through them (owner, 2026-11-24), at the cost of a low-weight environment-access alert.
- **Unchanged from ADR 0077.** The scoped name and `living-docs` bin, trusted publishing with staged releases and provenance, the sha256 check, version lockstep with `VERSION`, and the four supported targets.

Rejected alternatives:

- **Keep the install script (ADR 0077 as is).** The score stays at 65 and every install runs a network download the shim can do later.
- **One optional dependency per platform** (the esbuild and Biome pattern), which ADR 0067 rejected. It removes the network access too, but publishes four more packages per release, each needing its own trusted publisher and staged approval; worth it only if the score itself becomes a requirement.
- **Remove the environment overrides too.** Clears one more low-weight alert, but the fixture tests would need another injection seam (owner, 2026-11-24).

## Consequences

**Easier / gained:**
- `npm install` runs no package code: the install-script alert is gone, and a user installing with `--ignore-scripts` (a common hardening default) gets the same behavior as everyone else.
- One download path instead of two, so the shim is the only code that fetches.

**Harder / accepted trade-offs:**
- The first run needs network access to GitHub; an install made offline fails at first use, not at install.
- The first run is slower by one download (about 17 MB).
- A global install by root caches the binary per user, so each user downloads it once.
- The network-access alert stays, since the shim still fetches.

**Follow-ups:**
- After the next release goes live on npm, record the new Socket supply-chain score in this ADR's Verification.

## Verification

**Implementation impact:** `npm/package.json`, `npm/scripts/postinstall.js` (removed), `npm/lib/asset.js`, `npm/bin/living-docs.js`, `npm/README.md`, `scripts/tests/npm/run.sh`.

**Verification criteria:**
- The packed `package.json` has no `preinstall`, `install` or `postinstall` script, and the tarball holds no `scripts/` file.
- `npm install` of the tarball exits 0, writes no binary and requests nothing from the stub server; the first run downloads once, prints the asset's output, and the second run requests nothing.
- A bad checksum on first run exits non-zero, leaves no binary or partial file, and names `install.sh`.
- With the package directory read-only, the first run writes the binary under `$XDG_CACHE_HOME/living-docs/<version>/` and runs it.
- An unsupported platform fails on first run listing the four targets, and the install itself exits 0.
- `scripts/tests/npm/run.sh` checks each criterion above and runs in `make check`.
