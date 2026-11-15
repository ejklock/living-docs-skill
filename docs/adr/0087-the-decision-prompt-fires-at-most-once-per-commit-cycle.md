---
type: ADR
title: The decision prompt fires at most once per commit cycle
description: Change the opt-in Stop hook's state key from the changed content to HEAD after QA showed it re-prompting on every coding turn, superseding only the trigger of ADR 0083
owner: Evaldo Klock
status: Superseded
superseded_by: 0089
timestamp: 2026-11-15T22:15:56Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0089](0089-the-decision-reminder-is-a-non-blocking-posttooluse-nudge-once-per-commit-cycle.md). Run `living-docs read records` for what is in force.

# 0087. The decision prompt fires at most once per commit cycle

## Context

[ADR 0083](/adr/0083-an-opt-in-stop-hook-prompts-the-agent-to-record-a-material-choice-in-the-active-issue-before-the-turn-ends.md) prompts the agent at the end of a turn when code changed and no record did, at most once per distinct change set: the hook keys its state on the changed paths and their content. A QA pass on 2026-11-15 replayed a scripted session against the shipped hook ([issue 0126](/issues/0126-implement-adr-0083-the-opt-in-decision-prompt-stop-hook-and-its-installation.md)). After the agent answered "no material decision", the very next turn that edited any code prompted again, because the content changed; only touching a record silenced it. A session of twenty coding turns with no decision would prompt about twenty times, which the owner judged irritating enough to train the agent to answer by reflex — the failure mode ADR 0083's kill criterion counts. The cost per run was 120–143 ms on a 3,279-file repository, so cost is not the problem; frequency is.

## Decision

We will prompt at most once per commit cycle: the hook keys its state on the current `HEAD` commit (or the absence of one) instead of the changed paths and content.

- The other trigger conditions stay: opt-in through `LIVING_DOCS_DECISION_PROMPT=1`, non-bundle paths changed, no bundle path changed, `stop_hook_active` false, a payload carrying `stop_hook_active`.
- Once the hook has prompted for a `HEAD`, it stays silent for that `HEAD` whatever else changes; after a commit (a new `HEAD`) it may prompt once more.
- The message, the opt-in, the installation and the kill criterion of ADR 0083 are unchanged.

This supersedes only the trigger bullet of ADR 0083.

Rejected alternatives:
- **Once per distinct change set** (ADR 0083 as accepted). It re-prompts on every coding turn after a "no material decision" answer.
- **Once per session.** Quietest, but a long session that makes a decision after its first commit is never asked again, and the hook has no reliable session identity across harness restarts.

## Consequences

**Easier / gained:**
- At most one prompt between commits, so an answered prompt is not repeated within the same unit of work.

**Harder / accepted trade-offs:**
- A material choice made after the prompt and before the next commit is not prompted for; the session line and the commit-time review remain the backstop.
- A repository that never commits during a long session gets one prompt for the whole session.

**Follow-ups:**
- An issue to change the state key in `decision-prompt.sh` and its tests.

## Verification

**Implementation impact:** `skills/living-docs/hooks/decision-prompt.sh`, `cli/tests/decision_prompt.rs`.

**Verification criteria:**
- Code edit → prompt; further code edits on the same `HEAD` → silent; commit, then a code edit → one prompt.
- A repository with no commits prompts once and then stays silent until its first commit.
- Every other ADR 0083 criterion still holds.
