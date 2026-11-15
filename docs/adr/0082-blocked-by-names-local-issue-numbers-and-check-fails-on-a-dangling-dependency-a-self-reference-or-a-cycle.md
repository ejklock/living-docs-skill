---
type: ADR
title: blocked_by names local issue numbers and check fails on a dangling dependency, a self-reference or a cycle
description: Fix the referent of the issue blocked_by key as local issue numbers, refuse non-numbers at write time, fail check on a dependency graph that cannot be honored, and keep it intact across renumbering
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-15T16:01:28Z
---

# 0082. blocked_by names local issue numbers and check fails on a dangling dependency, a self-reference or a cycle

## Context

An issue declares what it waits on in the `blocked_by` frontmatter key. Since [issue 0081](/issues/0081-set-cannot-write-the-tracker-labels-and-blocked-by-keys-that-the-issue-workflow-puts-in-issue-frontmatter.md), `write set <issue> blocked_by <items>` writes it as a flow list, but nothing reads it: `living-docs-core/src/commands/set/issue_keys.rs` accepts any comma-separated text, no pass under `living-docs-core/src/check/` mentions the key, and `skills/living-docs/rules/issue-workflow.md` calls the issue frontmatter "tracker-specific". The key has no declared referent, so no tool can tell a valid dependency from a typo.

In practice the bundles already use one referent. A read-only probe on 2026-11-15 found that every `blocked_by` item is a bare issue number: 11 issues in this bundle and 67 issues (95 items) in the relent bundle. Every item resolves to an issue in the same bundle, none is a self-reference, and neither graph has a cycle; Kahn layering gives 4 dependency layers here and 9 in relent. The gap is ahead, not behind: `write set <TYPE/NNNN> number next` ([ADR 0069](/adr/0069-duplicate-record-numbers-fail-check-and-set-number-renumbers-a-record-with-its-links-and-index.md)) rewrites links and the index row, but `living-docs-core/src/commands/set/renumber.rs` never touches `blocked_by`, so renumbering a blocker leaves every dependent pointing at a number that no longer exists, silently.

The owner wants issues grouped into waves to scope work ([issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md), evidence in [research 0005](/research/0005-engineering-documentation-in-ai-assisted-development-living-architecture-domain-language-and-rules-decision-capture-issue-waves-and-a-local-html-view.md)). Any wave plan derived from dependencies is only as sound as the dependency graph, and an unvalidated graph can hold a dangling edge or a cycle that makes a plan meaningless.

## Decision

We will define `blocked_by` as a list of local issue numbers and make `check` fail on a dependency graph that cannot be honored.

- **Referent.** Each item is the number of an issue record in the same bundle, written as digits (`12` and `0012` name the same issue). A tracker identifier belongs in the `tracker` key, never in `blocked_by`.
- **Write time.** `write set <issue> blocked_by <items>` refuses an item that is not a number, naming the item. It does not require the blocker to exist yet, because a merge can bring it.
- **Check.** A new violation, reported per issue, for: an item that resolves to no issue in the bundle; an issue that lists itself; and a cycle, naming every issue on it. A blocker in any status resolves; a closed blocker is a satisfied dependency, not an error. The pass honors `check --changed-files` ([ADR 0062](/adr/0062-check-changed-files-scopes-the-gate-to-the-records-a-commit-touches-so-a-brownfield-bundle-can-arm-the-hook.md)) for the per-issue findings; a cycle is reported when any issue on it is in scope.
- **Renumber.** `write set <issue> number next` rewrites the renumbered issue's number in every other issue's `blocked_by`, in the same run as its links and index row.
- **Waves.** Release grouping stays the existing `labels` convention. A derived wave plan (Kahn layers of open issues) is not part of this decision; it waits for a consumer that asks for it, and would arrive as a `read records` flag, never a new verb ([ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md)).

Rejected alternatives:
- **Tracker identifiers as the referent.** They match how a team talks about work, but `check` cannot resolve them offline, so the graph could never be validated; the `tracker` key already holds the link to the tracker.
- **Free text, as today.** It costs nothing, but the key stays unreadable by any tool and a renumber keeps breaking it silently.
- **`TYPE/NNNN` references.** They would let an issue wait on an ADR, but acceptance of an ADR is already expressed by an issue declaring it implements the ADR ([ADR 0079](/adr/0079-stale-proposed-counts-only-an-issue-that-declares-it-implements-the-adr.md)); a prefix on every item adds noise to a key that only ever names issues, and both bundles hold zero non-issue items.
- **An advisory instead of a violation.** [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) removed advisories without a true positive and promoted mechanical defects to violations; a dangling dependency or a cycle is a mechanical defect with an exact oracle, not a matter of judgment.
- **A `wave` frontmatter key or an epic record type.** A `wave` key duplicates what a label already carries; an epic record is a second record per scope that goes stale, the reason ADR 0064 rejected a plan record.

## Consequences

**Easier / gained:**
- A dependency an issue declares is guaranteed to name a real issue, and the graph is guaranteed acyclic, so any wave plan built on it later is sound.
- Renumbering an issue no longer breaks its dependents.

**Harder / accepted trade-offs:**
- A bundle that put tracker identifiers or prose in `blocked_by` fails `check` until it moves them to `tracker` or the body. The two known bundles have none; an unknown consumer bundle may, and gets no grace window, as with ADR 0057's deleted type. `check --changed-files` limits the failure to the issues a commit touches.
- The new pass finds nothing in either known bundle today: its value is preventive, against the renumber gap and future edits. That is accepted because the oracle is exact and the cost is one pass over frontmatter already parsed.
- The issue-workflow rule stops calling `blocked_by` tracker-specific.

**Follow-ups:**
- An issue to implement the check pass, the write-time refusal, the renumber rewrite and the rule text.

## Verification

**Implementation impact:** `living-docs-core/src/check/` (a new dependency pass), `living-docs-core/src/commands/set/issue_keys.rs`, `living-docs-core/src/commands/set/renumber.rs`, `skills/living-docs/rules/issue-workflow.md`, `cli/src/args/help.rs` (the key's help text).

**Verification criteria:**
- Fixture bundles: an item naming a missing issue, a self-reference and a three-issue cycle each make `check` exit 1 with a finding naming the issue (and, for the cycle, every member); an issue blocked by a closed issue passes; `12` and `0012` resolve to the same issue.
- `write set issue/NNNN blocked_by "12, PROJ-9"` exits 2 naming `PROJ-9` and leaves the record unchanged.
- After `write set issue/0012 number next`, every issue that listed `12` lists the new number, and `check` passes.
- `living-docs check` over this bundle and over a copy of the relent bundle stays green.
