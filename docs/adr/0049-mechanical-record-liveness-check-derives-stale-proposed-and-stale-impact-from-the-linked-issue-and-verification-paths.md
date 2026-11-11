---
type: ADR
title: "Mechanical record liveness: check derives stale-proposed and stale-impact from the linked issue and Verification paths"
description: check derives a liveness signal for ADR/BDR mechanically — Proposed with a terminal linked issue, Accepted with a dead Implementation-impact path — as advisories with a --liveness summary; currency has an oracle where materiality does not
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T11:51:50Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0049. Mechanical record liveness: check derives stale-proposed and stale-impact from the linked issue and Verification paths

## Context

A record's `status` is whatever its author last wrote; nothing cross-references it with the world it describes. On a strict-mode consumer, 38 of 142 ADRs sit `Proposed` while their implementation issues are already closed with code merged ([issue 0035](/issues/0035-provenance-review-queue-check-flags-records-whose-referenced-source-was-superseded-or-changed-status.md) covered a source *moving*; nothing covers a record's own *currency*). A stale record is worse than a missing one, because the agent that reads the corpus trusts it. `enforcement-modes.md` places doc-trail judgement with the agent because materiality has no sound oracle — but currency does: the linked issue's status and the filesystem. The checkable part belongs in the tool.

## Decision

We will add a `check::liveness` pass that classifies each ADR/BDR from evidence the record already carries, emitting advisories only (exit code unchanged, same posture as `SIZE`/`MOVED-SOURCE`):

1. **stale-proposed** — a record still at its seed status (`Proposed`/`Draft`) that links a bundle-relative issue whose status is terminal for its type (`closed`/`done`, registry-sourced) or `Superseded`.
2. **stale-impact** — an `Accepted`/`Implemented` record whose `Implementation impact:` names a literal repository path (a `/`-bearing token with a source extension, resolved against the bundle's parent) that no longer exists and is not annotated `(removed)`/`(deleted)`.
3. **contract vs narrative** — an `Accepted` record with a `## Verification` block is a *contract*; without one it is *narrative*. Not a finding: a classification surfaced only in the `--liveness` summary and the reusable API, so downstream serving (issue #55) can rank contracts above narrative.

`--liveness` adds a summary of the four counts. The classification is exported as `check::liveness::classify` so the effective view (#55) and `why` (#56) exclude stale records from what they serve without re-deriving the rule.

## Consequences

**Easier / gained:**
- The one class of doc rot with a mechanical oracle becomes visible at `check` time, and downstream read verbs get one source of truth for "is this record still live".

**Harder / accepted trade-offs:**
- `stale-impact` reads free-form prose, so it is deliberately conservative (literal paths only): it under-reports rather than flag a descriptive impact line. Accepted — a false stale-impact would erode trust in the advisory faster than a missed one.
- `narrative` is not surfaced per-record; most advisory ADRs are narrative and a line each would be noise. Accepted — the signal that matters is *stale*, not *narrative*.

**Follow-ups:**
- A later decision, informed by the false-positive rate on real corpora, decides whether liveness gates rather than advises.

## Verification

**Implementation impact:** `living-docs-core/src/check/liveness.rs` (new), `living-docs-core/src/check/mod.rs` (wiring + `--liveness` summary), `cli/src/commands/check.rs`, `cli/src/args.rs` (`--liveness`), `skills/living-docs/rules/check.md`.

**Verification criteria:**
- `check` on a fixture with a `Proposed` ADR linked to a `closed` issue prints a stale-proposed advisory and still exits 0; a fixture whose Verification lists a missing path prints stale-impact.
- `check docs` over this repository stays green and reports its own liveness counts under `--liveness`.
