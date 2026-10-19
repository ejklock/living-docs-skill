---
type: ADR
title: An npm package wraps the released binary as a second distribution channel so npx living-docs resolves
description: A living-docs package on npm carries a postinstall that fetches the matching GitHub release asset, sha256-verified, so npx living-docs and npm install -g living-docs work where install.sh is unknown.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-19T13:13:52Z
---

# 0067. An npm package wraps the released binary as a second distribution channel so npx living-docs resolves

## Context

[ADR 0028](/adr/0028-the-release-binary-is-the-unit-of-distribution-install-sh-only-bootstraps-it-and-every-placement-becomes-a-cli-verb.md) made the release binary the unit of distribution and [ADR 0041](/adr/0041-make-cli-install-fetches-the-released-binary-and-install-sh-resolves-the-latest-release-by-default.md) made `install.sh` fetch it from the GitHub release, sha256-verified. The release workflow publishes five platform assets with checksums and a job that verifies every one of them.

The usage analysis over the owner's agent transcripts recorded 100 attempts of `npx living-docs`, 15 of them answered by npm's 404 and the rest by a shell fallback the agent wrote around the absence. An agent looks for a CLI where the ecosystem it works in keeps CLIs: in a Node project, that is npm. `install.sh` is unknown to it, and `cargo install` requires a toolchain the host does not have. The name `living-docs` is unclaimed on npm (`npm view living-docs` returns 404 on 2026-10-19).

## Decision

We will publish `living-docs` on npm as a thin wrapper around the released binary, a second distribution channel beside `install.sh`, never a second implementation.

- **Package shape.** `npm/package.json` with a `bin` entry `living-docs`, zero runtime dependencies, Node 18 or later. A `postinstall` script maps the platform and architecture to the release asset name used by `.github/workflows/release.yml`, downloads `living-docs-<target>` and its `.sha256` from `github.com/ejklock/living-docs-skill/releases/download/v<version>/`, verifies the checksum, and places the executable inside the package directory. A checksum mismatch or a download failure removes any partial file and fails the install with the `install.sh` command as the fallback.
- **Bin shim.** `bin/living-docs.js` executes the binary with inherited stdio and propagates its exit code. When the binary is missing because the install ran with `--ignore-scripts`, the shim performs the same download on first run.
- **Version lockstep.** The package version equals `VERSION`; `scripts/check-version.sh` fails a release whose `npm/package.json` drifts.
- **Publishing.** A `publish-npm` job in the release workflow runs after the asset-verification job, with `npm publish --provenance`, and is skipped when the `NPM_TOKEN` secret is absent, so the Rust release never waits on npm.

Rejected alternatives:

- **One optional dependency per platform** (the esbuild and Biome pattern). It publishes six packages per release and duplicates the assets GitHub already serves with checksums; it wins only when postinstall scripts are forbidden, which the lazy download in the shim covers.
- **A Node reimplementation of the verbs.** Two implementations of one contract drift; the binary is the implementation.
- **`npx github:ejklock/living-docs-skill`.** It compiles at install time and needs a Rust toolchain on the host.
- **Homebrew or cargo-binstall only.** Other audiences; the data shows where agents look.

## Consequences

**Easier / gained:**
- `npx living-docs check` works in any repository with Node, and a Node project can pin the gate as a devDependency so CI needs no extra install step.

**Harder / accepted trade-offs:**
- A second artifact to version, test and verify on every release, and a public package name that is a standing commitment.
- Installing needs network access to github.com at postinstall or first run, the same trust and checksum `install.sh` relies on.

**Follow-ups:**
- [Issue 0061](/issues/0061-npm-package-living-docs-wraps-the-release-binary-and-the-release-workflow-publishes-it.md) implements the package, the shim, the version gate and the workflow job.
- The owner creates the npm account and the `NPM_TOKEN` repository secret; until then the job is skipped.
- The README installation section lists the npm channel beside `install.sh`.

## Verification

**Implementation impact:** `npm/package.json`, `npm/bin/living-docs.js`, `npm/scripts/postinstall.js`, `npm/lib/`, `.github/workflows/release.yml`, `scripts/check-version.sh`, `scripts/tests/npm/`, `README.md`.

**Verification criteria:**
- With the download stubbed to a local asset and checksum, `node npm/bin/living-docs.js --version` prints the release version.
- A stubbed checksum mismatch leaves no binary in the package directory and exits non-zero naming `install.sh`.
- After an install with `--ignore-scripts`, the first run downloads the binary and runs the verb.
- `scripts/check-version.sh` fails when `npm/package.json` and `VERSION` differ.
- The fixture runner `scripts/tests/npm/run.sh` is part of `make check`.
