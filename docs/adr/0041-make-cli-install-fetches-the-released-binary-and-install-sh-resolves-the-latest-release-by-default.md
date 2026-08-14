---
type: ADR
title: make cli-install fetches the released binary and install.sh resolves the latest release by default
description: <One sentence — the decision and its scope.>
status: Accepted
timestamp: 2026-08-14T13:49:32Z
---

# 0041. make cli-install fetches the released binary and install.sh resolves the latest release by default

## Context

[ADR 0028](/adr/0028-the-release-binary-is-the-unit-of-distribution-install-sh-only-bootstraps-it-and-every-placement-becomes-a-cli-verb.md)
made the released binary the unit of distribution, but the shipped code split CLI
installation across two disagreeing paths. `./install.sh cli` downloads a release asset to
`~/.local/bin`. `make cli-install` runs `cargo install --path cli --force` into
`~/.cargo/bin`.

This split reintroduced the exact drift ADR 0028 set out to remove. `cargo install` writes
to `~/.cargo/bin`, which is often absent from `PATH`; `./install.sh cli` writes a different
binary to `~/.local/bin`. Two binaries, two versions, and the one first on `PATH` wins in
silence. A developer who ran `make cli-install` after a release saw a stale
`~/.local/bin/living-docs` shadow the freshly built `~/.cargo/bin` copy — `living-docs
--version` reported the old version with no error.

ADR 0028 also stated that `install.sh` "resolves the latest release by default;
`LIVING_DOCS_VERSION` pins an exact tag." The shipped `install.sh` never did this. It read
the checked-out `VERSION` file for the tag, so it required a clone and could not fetch the
newest release from a bare `curl` pipe.

## Decision

We will collapse both install paths onto one artifact and one version rule.

- `make cli-install` stops compiling. It becomes a thin wrapper over `./install.sh cli`, so
  every CLI install comes from a published release asset placed in `~/.local/bin`. The
  `cargo install` line is removed from the target.
- `make build` stays the only local-compile path. It produces `target/release/living-docs`
  and installs nothing.
- `./install.sh cli` resolves the version to fetch in this order: `LIVING_DOCS_VERSION`,
  when set, pins an exact tag; otherwise the script queries the GitHub releases API for the
  latest published tag. Resolution uses `curl` against
  `https://api.github.com/repos/<repo>/releases/latest` with no `jq` or `gh` dependency, so
  the script stays curl-pipeable. The existing sha256 verification and build-from-source
  fallback are unchanged.

## Consequences

**Easier / gained:**
- One binary path and one version rule. "Which `living-docs` am I running" has a single
  answer, and `make cli-install` can no longer plant a shadow in `~/.cargo/bin`.
- `./install.sh cli` fetches the newest release without a checkout, honoring ADR 0028's
  curl-pipeable intent.
- Developers and users share one install command shape.

**Harder / accepted trade-offs:**
- `make cli-install` now needs network access and a published release. It no longer installs
  an unreleased local build. A developer testing an uncommitted change runs `make build` and
  executes `target/release/living-docs` directly.
- This revises two ADR 0028 verification criteria: "No `Makefile` target writes into
  `~/.local/bin`" and "a `cli-install` that installs the local build for development." ADR
  0028's core — the released binary is the unit of distribution — stands; only the
  `cli-install` placement and the version-resolution mechanism change. This ADR refines ADR
  0028; it does not supersede it.
- A pre-existing `~/.cargo/bin/living-docs` from an older `make cli-install` can still shadow
  the released binary until a person removes it. That is an operational cleanup, not a code
  path.

**Follow-ups:**
- ADR 0028's promised shadow detection — the installer reports a `living-docs` earlier on
  `PATH` than its destination — stays unbuilt and is worth a separate issue.

## Verification

**Implementation impact:** `install.sh` (the `install_cli` version resolution), `Makefile`
(the `cli-install` target and its help text), `scripts/tests/install/` (a new fixture
suite), `README.md`, `CONTRIBUTING.md`.

**Verification criteria:**
- `make cli-install` contains no `cargo install`; it invokes `./install.sh cli`.
  Grep-checkable.
- With `LIVING_DOCS_VERSION` unset, `./install.sh cli` queries the `releases/latest`
  endpoint and downloads the returned tag's asset; with it set, the script downloads that
  exact tag and never queries `releases/latest`.
- Fitness function: a negative fixture suite under `scripts/tests/install/`, driven by a
  stubbed `curl` on `PATH` with no network, covering latest-tag resolution, the
  `LIVING_DOCS_VERSION` pin, a checksum mismatch, and an unknown platform — mirroring
  `scripts/tests/verify-release-assets/`.
