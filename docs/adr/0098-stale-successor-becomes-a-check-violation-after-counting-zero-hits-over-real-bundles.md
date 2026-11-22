---
type: ADR
title: stale-successor becomes a check violation after counting zero hits over real bundles
description: A record with superseded_by and a status other than Superseded now fails check; the other advisory kinds stay advisories.
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-22T23:14:37Z
---

# 0098. stale-successor becomes a check violation after counting zero hits over real bundles

## Context

[ADR 0095](0095-a-new-check-rule-starts-as-an-advisory-and-becomes-a-violation-only-by-a-later-adr-that-counts-it-over-real-bundles.md) ships four new `check` rules as advisories and promotes one to a violation only by a later ADR that counts it over real bundles. `stale-successor` fires when a record carries a non-empty `superseded_by` and a status other than Superseded: `read records` then lists it in force beside its own successor, which breaks the North Star of one decision in force per question.

Counted over the 42 measurable bundles of the 2026-11-20 survey with the binary of issue 0149: 0 hits. The CLI path that wrote this state (`write set status` on a superseded record) is refused since issue 0149, so the only remaining source is a hand edit.

## Decision

We will report `stale-successor` as a violation of `check`, under the same name, instead of an advisory. The other three kinds of ADR 0095 (`status-vocabulary`, `record-type`, `timestamp`) and `path-missing` stay advisories.

## Consequences

- A hand edit that puts a superseded record back in force fails the pre-commit gate and CI, instead of passing with a note; the status refusal already tells the author how to undo a supersede by hand.
- No surveyed bundle turns red on upgrade (0 hits); a bundle outside the survey that holds the state fails until its author fixes it.
- `check --json` moves the finding from `advisories` to `violations`; a consumer that filtered on the advisory kind reads the violation instead.
- Fitness function: a test drives `check` on a bundle holding the state and asserts a non-zero exit with the violation named, and the eval scenario that revives a superseded ADR keeps passing.
