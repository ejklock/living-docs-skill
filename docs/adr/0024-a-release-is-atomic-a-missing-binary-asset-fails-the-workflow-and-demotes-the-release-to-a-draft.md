---
type: ADR
title: "A release is atomic: a missing binary asset fails the workflow and demotes the release to a draft"
description: A release job that publishes an incomplete asset set fails loudly and demotes the release to a draft, so install.sh never sees a half-published tag.
owner: Evaldo Klock
status: Superseded
superseded_by: 0025
timestamp: 2026-07-29T16:08:56Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0025](0025-releases-are-born-draft-and-earn-publication-by-passing-the-asset-gate.md). Run `living-docs read records` for what is in force.

# 0024. A release is atomic: a missing binary asset fails the workflow and demotes the release to a draft

## Context

`install.sh` already downloads a prebuilt `living-docs-<target-triple>` binary from the
GitHub Release matching `VERSION`, verifies its `.sha256`, and only builds from source when
that download fails. The `Release` workflow already cross-compiles four targets and uploads
each binary plus its checksum. On paper the "no Makefile needed" path is complete.

In practice it has never worked. The `v0.7.0` release run failed at `make check` —
`make test-fixtures` rejected two of the repo's own frontmatter fixtures (the bug ADR 0022
later fixed). The `release` job died, `release-binaries` `needs: release` and was therefore
skipped, and the tag was published carrying **zero assets**. `v0.8.0` was never tagged at all.

Nothing detected this. The failure surfaced only as a red run nobody read, while
`install.sh` degraded exactly as designed: `release asset unavailable for <triple>; falling
back to build from source`. The consumer experience — "it still needs the Makefile" — is the
*symptom*; the cause is a release that publishes successfully while being incomplete.

Two structural gaps make a partial release possible and silent:

1. `release-binaries` uses `fail-fast: false`, so one broken target does not stop the other
   three from uploading. That is the right build policy — we want to learn which targets
   broke — but it means a green-ish run can still leave a release short of assets.
2. No step ever asserts the *published* result. Each job verifies its own step; nothing
   verifies the release as a whole.

The asset names are a contract: `install.sh` constructs the download URL from
`living-docs-$triple` and `$asset.sha256`. A rename on either side breaks adoption silently,
and no test covers that agreement.

## Decision

We will treat a release as **atomic**: it is complete or it is not a release.

**1. A `verify-release-assets` job asserts the published asset set.** It runs after
`release-binaries` with `if: always()`, so it executes precisely in the cases worth catching
— a skipped or partially failed matrix. It reads the published assets with
`gh release view "$GITHUB_REF_NAME"` and compares them against a hard-coded expected list:
the skills zip and its `.sha256`, plus `living-docs-<triple>` and `living-docs-<triple>.sha256`
for each of the four supported targets. Any missing name fails the job, naming every absent
asset.

**2. An incomplete release is demoted to a draft, not left published.** Before failing, the
job runs `gh release edit --draft`. A draft is invisible to `install.sh` (its download 404s
the same way an absent asset does) and, unlike deletion, preserves the run for diagnosis and
lets a re-run promote it. Rationale: the window between "assets partially uploaded" and "a
human notices the red run" is exactly when a consumer installs a broken version.

**3. The gate lives in `scripts/verify-release-assets.sh`, not inline in the YAML.** Inline
`run:` blocks are invisible to every linter this repo owns — `make check` shell-checks
`install.sh` and `scripts/check-version.sh` and never parses workflow YAML. A real script
file is syntax-checked by `make check`, runnable locally against any existing tag, and
reviewable like any other code. The workflow job becomes a one-line invocation.

**4. The gate verifies integrity, not just presence.** For each expected binary it downloads
the asset and its `.sha256` and verifies the checksum with the same comparison `install.sh`
performs. Presence catches the skipped-job failure; the checksum catches a truncated or
mismatched upload. This is the only place the asset-naming contract is executed end-to-end
rather than asserted in a comment.

We will not change `install.sh`. Its fallback-to-source behavior is correct and already
announces itself; the defect is on the publishing side.

## Consequences

**Easier / gained:**
- A tag either yields four verified binaries or does not yield a usable release at all.
- The asset-naming contract between the workflow and `install.sh` becomes executable instead
  of a comment in the workflow header.
- `install.sh` stops being a de-facto source builder for users on supported platforms, which
  is the outcome originally intended when the download path was added.
- The gate can be run by hand against any past tag to audit what that release actually
  shipped, which is how `v0.7.0`'s emptiness would have been found.

**Harder / accepted trade-offs:**
- Releasing gets slower: the gate downloads every binary before the release is trusted.
  Accepted — it runs once per tag and the assets are small.
- A transient GitHub API or download failure can demote an otherwise good release to a draft.
  Accepted: a draft is recoverable by re-running the job, whereas a silently broken published
  release is not recoverable at all once someone has installed from it.
- The gate needs `contents: write` to demote, which the workflow already grants.
- The supported target triples are now named in two places — the build matrix and the gate's
  expected list. Accepted rather than deriving one from the other: parsing the matrix out of
  the YAML at runtime trades a visible, greppable list for fragile introspection. The gate
  fails loudly if they disagree, which is the behavior that matters.

**Follow-ups:**
- Consider having the gate promote a verified draft to published, inverting the default so a
  release is born draft and earns publication.

## Verification

**Implementation impact:** `scripts/verify-release-assets.sh` (new), `.github/workflows/release.yml`
(one new job invoking it), `Makefile` (`bash -n` coverage for the new script).

**Verification criteria:**
- `scripts/verify-release-assets.sh <tag>` exits 0 against a release carrying all ten expected
  assets, and exits non-zero naming every absent asset when any is missing.
- On failure the script demotes the release with `gh release edit <tag> --draft` before
  exiting non-zero; on success it leaves the release published and untouched.
- For every expected binary the script recomputes the SHA-256 of the downloaded asset and
  compares it to the published `.sha256`; a mismatch fails the script.
- `verify-release-assets` runs with `if: always()` after `release-binaries`, so a skipped or
  partially failed matrix still reaches the gate.
- Fitness function: `make check` runs `bash -n scripts/verify-release-assets.sh`, so the gate
  cannot be merged unparseable — the same coverage `install.sh` and `check-version.sh` get.
