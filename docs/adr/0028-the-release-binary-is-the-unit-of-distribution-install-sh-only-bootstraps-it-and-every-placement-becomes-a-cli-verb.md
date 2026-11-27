---
type: ADR
title: "The release binary is the unit of distribution: install.sh only bootstraps it and every placement becomes a CLI verb"
description: Installation collapses to one artifact — the released binary — fetched by a bootstrap script whose only job is that, with every harness and hook placement served from the binary's embedded corpus as a CLI verb.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-07-30T22:33:04Z
---

# 0028. The release binary is the unit of distribution: install.sh only bootstraps it and every placement becomes a CLI verb

## Context

There are eleven ways to install this project and sixteen places it can write. That is not
an inventory problem, it is two mental models sharing one script.

`install.sh` has nine modes. Eight of them **copy files out of the checked-out working
tree** — the three skill directories into a harness path, or a generated pointer file into
`.cursor/rules/` or `.github/instructions/`. Only `install.sh cli` downloads a release
asset. So the documented way to install a skill requires cloning the repository first, and
what you get is whatever that clone happens to contain rather than what was released.

The `Makefile` then wraps the same script in fourteen targets: a second entry point to the
same actions, with its own names and its own help text, drifting independently. And the two
paths that install the binary disagree — `install.sh cli` writes `~/.local/bin/living-docs`
while `make cli-install` runs `cargo install` into `~/.cargo/bin`. Two binaries, two
versions, no gate between them, and whichever comes first on `PATH` wins silently. That is
the drift class [ADR 0027](0027-every-rule-keyed-by-doc-type-becomes-a-registry-field-and-glossary-is-not-a-doc-type.md)
and `check-version.sh` exist to prevent, reappearing one level up.

The way out is already in the repository, applied once and stopped.
[ADR 0014](0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md)
(superseded by [ADR 0017](0017-skill-md-stubs-are-pure-routers-the-spine-and-all-detail-move-to-cli-topics.md))
embedded the skills corpus in the binary. ADR 0023 (Deprecated, no successor — see ADR 0059; the `hooks install` channel survives, the write-gate does not)
used that to make `living-docs hooks install` materialize hook scripts from the embedded
copy into a target project, with no working tree involved anywhere. The binary is already a
self-contained carrier of everything this project distributes. Only one artifact class was
ever taught to come out of it.

## Decision

We will make the released binary the unit of distribution. Nothing installs by copying out
of a checked-out tree.

> Amended by [ADR 0108](0108-a-claude-code-plugin-and-marketplace-at-the-repository-root-ship-the-skills-the-mcp-server-and-the-session-hook.md): the Claude Code plugin installs from a copy of the tree, an exception for that one channel.

**One bootstrap.** `install.sh` keeps exactly one job: detect the platform triple,
download `living-docs-<triple>` from a GitHub release, verify its published `.sha256`,
place it in `~/.local/bin` (overridable), and stop. No harness modes, no file copying, no
knowledge of what a skill is. It becomes safe to pipe from `curl`, which is the shape a
user expects and the shape that does not require a clone. It resolves the latest release by
default; `LIVING_DOCS_VERSION` pins an exact tag, because a CI job that cannot pin is not
reproducible.

**Every placement is a CLI verb**, served from the embedded corpus and joining
`hooks install`: `living-docs skill install [--harness <name>] [--project]` covers the six
harness targets that are file copies or generated pointer files today, and
`living-docs companions install` covers the Matt Pocock clone — the one artifact that
cannot come from the binary, because it is a third-party repository, and therefore the one
that has to be a verb rather than an embedded asset.

**`make` is for people inside this repository.** Build, test, check, and a `cli-install`
that installs the local build for development. Every user-facing install target is removed.
Local tooling installs local; users install from a release.

The release keeps publishing the skills zip. It is redundant for installing, but it is the
only way to read the corpus without running a binary, and it costs one line in a gate that
already verifies it.

## Consequences

**Easier / gained:**
- One documented way in, with no clone: fetch the binary, then ask it to place things.
- Only the CLI knows where a harness keeps its skills, so adding a harness is one change in
  one language, testable by the suite that already tests `hooks install`.
- One binary path, so "which `living-docs` am I running" has an answer.

**Harder / accepted trade-offs:**
- Bootstrapping is strictly two steps now: the binary must exist before any skill can be
  placed. A single `curl | sh` no longer leaves a configured harness behind.
- Placement logic moves from shell into Rust. Harder to eyeball, but testable — which the
  shell placement never was.
- Contributors lose `./install.sh claude` against their own checkout. The `make` dev path
  covers them, but it is a real break for the people most likely to hit it first.
- A `~/.cargo/bin/living-docs` left by an earlier `make cli-install` can still shadow the
  released binary. The installer must detect and report that; it must not silently win and
  must not silently lose.

**Follow-ups:**
- An issue for the harness matrix itself: six targets, four of which are the same copy with
  a different destination. Whether that becomes a registry the way doc types did is a
  separate decision, and this ADR deliberately does not make it.

## Verification

**Implementation impact:** `install.sh`, `Makefile`, `cli/src/`, `README.md`,
`CONTRIBUTING.md`, `scripts/tests/install/`.

**Verification criteria:**
- `install.sh` contains no harness name, no `cp` out of the repository, and no reference to
  `skills/` — grep-checkable, and the sharpest single signal that the collapse happened.
- On a machine with no checkout, piping the script to `sh` yields a working `living-docs`,
  and `living-docs skill install --harness claude` then places the corpus.
- `LIVING_DOCS_VERSION=vX.Y.Z` installs exactly that tag; unset installs the latest.
- No `Makefile` target writes into a user harness directory or into `~/.local/bin`.
- The installer reports a `living-docs` that shadows its destination on `PATH`.
- Fitness function: a negative fixture suite for `install.sh` under `scripts/tests/install/`,
  mirroring `scripts/tests/check-version/`, driven by a stubbed `gh`/`curl` with no network —
  covering checksum mismatch, absent release, unknown platform, a pinned version, and the
  shadowing case. An installer without negative tests is precisely the failure the version
  gate just had: a gate nobody had ever seen fail.
