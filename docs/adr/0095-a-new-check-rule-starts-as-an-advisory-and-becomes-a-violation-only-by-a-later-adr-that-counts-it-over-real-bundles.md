---
type: ADR
title: A new check rule starts as an advisory and becomes a violation only by a later ADR that counts it over real bundles
description: Four rules ship as the advisory kinds status-vocabulary, record-type, timestamp and stale-successor; promotion to a violation is a separate, measured decision.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-20T16:30:40Z
---

# 0095. A new check rule starts as an advisory and becomes a violation only by a later ADR that counts it over real bundles

## Context

`check` owns frontmatter the CLI writes, yet four states the CLI itself can never produce pass it in silence when a record is edited by hand or left by an older release: a `status` outside the type's vocabulary (`done`, `Reverted`, `active`), a record in a typed directory whose `type` is not that directory's type (`type: BDR` under `adr/`), a record with no `timestamp`, and a record that carries `superseded_by` while its status is not Superseded. The last one makes `read records` list a record and its successor as both in force.

Turning the four into violations at once would repeat a failure this bundle already recorded. A strict `blocked_by` rule shipped as a violation, blocked four real bundles, and was walked back twice in one day ([ADR 0086](0086-blocked-by-fails-check-only-on-a-dangling-issue-number-a-self-reference-or-a-cycle-other-items-are-advice.md), then [ADR 0088](0088-every-blocked-by-finding-is-advice-and-none-fails-check.md)). A survey on 2026-11-20 found a missing `timestamp` in seven bundles (83 records in one of them) and `done` in routine use as an issue status.

## Decision

We will introduce a new `check` rule as an advisory, and promote it to a violation only by a later ADR that states how many records in real bundles the rule fires on. The four rules above ship now under the advisory kinds `status-vocabulary`, `record-type`, `timestamp` and `stale-successor`.

- `status-vocabulary`: a record of a registry type has a status that is neither in that type's vocabulary nor `Superseded`.
- `record-type`: a record file in a registry type's directory declares a different `type`.
- `timestamp`: a record of a registry type has no `timestamp`.
- `stale-successor`: a record has a non-empty `superseded_by` and a status other than Superseded.

Rejected alternatives:

- **Violations at once.** Several bundles would turn red on upgrade for states that predate the rule.
- **Only `stale-successor` as a violation.** It is the one that corrupts the in-force view, but no count over real bundles exists yet; the CLI path that produced it is closed in the same release, so the advisory covers the records already written.
- **A per-bundle switch that enforces a kind.** A second configuration channel beside the hook's environment variables ([ADR 0081](0081-the-distributed-pre-commit-hook-is-the-only-copy-and-environment-variables-opt-it-into-changed-only-and-owner-checks.md)); it waits for a bundle that asks for it.

## Consequences

**Easier / gained:**
- A new rule can ship without breaking a bundle on upgrade.
- Promotion is decided on a count, not on a guess.

**Harder / accepted trade-offs:**
- An advisory does not stop a commit, so a hand-typed `status: done` still lands until the kind is promoted.
- Each promotion costs one ADR.

**Fitness function:** one test per kind asserts the advisory fires on its state and that the report stays `ok`; the kind list test asserts the four names are rendered in `check --help`.
