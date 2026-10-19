---
type: Issue
title: npm package living-docs wraps the release binary and the release workflow publishes it
description: "Implements the ADR for the npm channel: an npm directory with the package, a postinstall that fetches and verifies the release asset for the platform, a bin shim that execs it, and a release job that publishes when the npm token is present."
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0061. npm package living-docs wraps the release binary and the release workflow publishes it

Implements [ADR 0067](/adr/0067-an-npm-package-wraps-the-released-binary-as-a-second-distribution-channel-so-npx-living-docs-resolves.md). A `living-docs` package on npm wraps the released binary so `npx living-docs` and `npm install -g living-docs` resolve, and the release workflow publishes it when the token is present.

### Scope

- `npm/package.json`: name `living-docs`, version equal to `VERSION`, `bin` pointing at the shim, `postinstall`, `engines.node >= 18`, no dependencies, `files` limited to the shim, scripts and lib.
- `npm/scripts/postinstall.js` and a shared `npm/lib/`: platform and architecture to release asset name (the matrix of `release.yml`), download of the asset and its `.sha256` from the GitHub release of the package version, checksum verification, executable bit, partial-file removal on any failure, an error naming `install.sh`.
- `npm/bin/living-docs.js`: runs the binary with inherited stdio and the caller's exit code; downloads on first run when the binary is missing. The download target is overridable through an environment variable so tests and air-gapped mirrors can point at a local server.
- `scripts/check-version.sh` covers `npm/package.json`.
- `.github/workflows/release.yml`: a `publish-npm` job after asset verification, `npm publish --provenance --access public`, skipped when `NPM_TOKEN` is absent.
- `scripts/tests/npm/run.sh`: fixtures with a local HTTP stub serving an asset and checksum; wired into `make check`.
- README installation section.

### Decision

- One package with a download step rather than per-platform packages, as the ADR decides; the lazy download in the shim covers `--ignore-scripts` installs.
- Node's built-in `fetch` and `crypto`; option not taken: a request library, which adds a dependency for one GET.

### Acceptance

- With the download URL pointed at a stub serving a fake asset and its checksum, `npm install` of the packed tarball places an executable and `node npm/bin/living-docs.js --version` prints the stub's output.
- A stub serving a wrong checksum fails the install, leaves no binary in the package directory and names `install.sh`.
- After `npm install --ignore-scripts`, the first run downloads and then runs.
- An unsupported platform fails with a message listing the supported targets.
- `scripts/check-version.sh` fails when `npm/package.json` differs from `VERSION` and passes when equal.
- `make check` runs the npm fixtures; the full suite passes.

### Plan

1. Package, lib and shim with the fixture runner against a local stub; red first.
2. Version gate and workflow job; README.

### Outcome

Delivered, uncommitted, in the working tree: a zero-dependency `living-docs` npm package under `npm/` (platform resolver, download of the release asset and its `.sha256`, hash verified before an atomic rename, bin shim with lazy download and exit-code propagation, postinstall naming `install.sh` as the fallback, `LIVING_DOCS_DOWNLOAD_BASE` and `LIVING_DOCS_PLATFORM` seams); `scripts/check-version.sh` covers `npm/package.json`; `publish-npm` job in the release workflow after asset verification, with provenance, skipped when `NPM_TOKEN` is absent; `make test-npm-gate` wired into `make check`; README and `npm/README.md` document the channel. Nothing was published. Proof: `bash scripts/tests/npm/run.sh` (good_checksum, bad_checksum, ignore_scripts, unsupported_platform all pass against a local stub), `scripts/tests/check-version/run.sh` (cases 17 and 18 new), `make check` exit 0, `se-gates check` pass. Owner actions before the first publish: claim the npm name and add the `NPM_TOKEN` repository secret.
