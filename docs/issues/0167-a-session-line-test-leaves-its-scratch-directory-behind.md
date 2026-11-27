---
type: Issue
title: A session line test leaves its scratch directory behind
description: hook_exits_zero_and_stays_one_line_without_a_cli creates target/tmp/session-context-no-cli and never removes it.
status: open
timestamp: 2026-11-27T20:46:57Z
---

## 0167. A session line test leaves its scratch directory behind

## Context

Found in the review of issue 0157 (2026-11-27). The test `hook_exits_zero_and_stays_one_line_without_a_cli` in `cli/tests/session_context_line.rs` creates `target/tmp/session-context-no-cli` and never removes it, and it rebuilds the script path by hand instead of using the file's run helper. It does no harm today.

## Decision

- None yet.

## Acceptance

- The test removes its scratch directory, also when an assertion fails, and uses a unique path per run.

## Scope

In: `cli/tests/session_context_line.rs`. Out: the hook script.
