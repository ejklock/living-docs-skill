---
type: Issue
title: check repeats the arm-the-hooks hint on every run
description: living-docs check prints the no pre-commit doc-gate is armed hint on every invocation in a repository without hooks, adding noise to each read of the result
status: closed
timestamp: 2026-11-16T04:48:58Z
---

## 0141. check repeats the arm-the-hooks hint on every run

### Scope

The QA walkthrough of 2026-11-16 showed `living-docs check` printing "no pre-commit doc-gate is armed in this repository; run `living-docs manage install hooks` to enforce it at commit" after every run in a project without hooks, three times in one short session. The hint (issue 0071) is useful once and noise after that, the kind of friction the owner asked to remove.

### Decision

- Print the arm-the-hooks hint only under `check --advisories`; a plain `check` stays silent about it. Deterministic, with no state written outside the bundle (owner, 2026-11-16). Not taken: once per day (needs a timestamp stored outside the bundle); keep it on every run.

### Acceptance

- The chosen rule is implemented and tested; a repeated `check` in a hook-less project does not repeat the hint on every run.

### Outcome

- The arm-the-hooks hint prints only under `check --advisories` (also with `--json`); the flag's help says so. Proof: `cli/tests/check_hook_hint.rs` › `hooks_hint_only_with_advisories` and its help test.
