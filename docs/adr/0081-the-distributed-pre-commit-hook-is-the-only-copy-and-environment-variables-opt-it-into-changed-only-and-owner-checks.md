---
type: ADR
title: The distributed pre-commit hook is the only copy and environment variables opt it into changed-only and owner checks
description: Keep one pre-commit hook, the distributed copy, tuned per project by LIVING_DOCS_CHECK_CHANGED_ONLY and LIVING_DOCS_REQUIRE_OWNER, with CI requiring owners on this repository's bundle.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-12T21:20:47Z
---

# 0081. The distributed pre-commit hook is the only copy and environment variables opt it into changed-only and owner checks

## Context

Two hand-kept copies of the pre-commit doc-gate exist. `skills/living-docs/hooks/pre-commit` is the copy `living-docs manage install hooks` writes into a project; it ends in `check --plain` with no option. This repository's `.githooks/pre-commit` has drifted from it: it carries the `LIVING_DOCS_CHECK_CHANGED_ONLY` opt-in that narrows the verdict to the staged records ([ADR 0062](/adr/0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md)), and it always passes `--require-owner`, which turns a missing owner from a warning into a violation ([ADR 0043](/adr/0043-owner-is-a-cli-owned-frontmatter-field-with-a-warn-then-error-ratchet-on-adr-and-bdr.md)).

A project that installs the hook therefore cannot arm the gate over a bundle with legacy debt, the opt-in ADR 0062 was written for, and has no way to require owners short of editing a file the next install overwrites. The duplicate-code gate already ignores `.githooks/**` to tolerate the two copies.

## Decision

We will keep one hook: the distributed `skills/living-docs/hooks/pre-commit`, and this repository's `.githooks/pre-commit` becomes byte-identical to what `manage install hooks` writes. Each project tunes the installed hook through environment variables only, never by editing it:

- `LIVING_DOCS_CHECK_CHANGED_ONLY=1` narrows the verdict to the staged records of the bundle, as the repository hook does today.
- `LIVING_DOCS_REQUIRE_OWNER=1` adds `--require-owner` to the check, with or without the changed-only opt-in.
- `LIVING_DOCS_BUNDLE` keeps naming the bundle directory, default `docs`.

Unset or any value other than `1` leaves the hook as it is today. Because this repository's hook stops forcing `--require-owner`, `make check` gains `check --require-owner --plain docs`, so CI requires owners on this bundle whatever a contributor's environment holds.

Rejected alternatives:

- **A git config key (`living-docs.requireOwner`).** It persists per clone, but it is a second configuration channel next to the environment variables the hook already reads, and every clone must set it again.
- **Always require owners in the distributed hook.** One less switch, but every project that installed the hook with records lacking an owner would start failing commits on upgrade.
- **Keep two copies.** The drift that motivated this record would recur, and the duplicate-code ignore would stay.

## Consequences

**Easier / gained:**
- A project that installed the hook can arm it over legacy debt and can require owners without editing it.
- One source for the hook; a test catches the repository copy drifting from the installed one, and `.jscpd.json` loses its `.githooks/**` ignore.

**Harder / accepted trade-offs:**
- Owner enforcement at commit time depends on each contributor's environment; CI is where it holds for everyone.
- The hook's contract now includes two environment variable names that a rename would break.

**Follow-ups:**
- [Issue 0114](/issues/0114-the-distributed-pre-commit-hook-lacks-the-changed-only-opt-in-the-repository-hook-has.md) implements this record.

## Verification

**Implementation impact:** `skills/living-docs/hooks/pre-commit`, `.githooks/pre-commit`, `Makefile`, `.jscpd.json`, the hook tests under `cli/tests/`, and the `procedure` and `check` guide topics where they teach the hook.

**Verification criteria:**
- In a project that ran `manage install hooks`, `LIVING_DOCS_CHECK_CHANGED_ONLY=1` fails a commit only on findings in the staged records; unset, the whole bundle is judged.
- With `LIVING_DOCS_REQUIRE_OWNER=1`, a staged ADR without an owner fails the commit; unset or `0`, it passes with a warning.
- Both variables together narrow to the staged records and require owners on them.
- `.githooks/pre-commit` is byte-identical to the installed copy, and a test fails when they differ.
- `make check` fails on an ADR in `docs/` that lacks an owner.
