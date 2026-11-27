---
type: Issue
title: The session-context tests read the host's git configuration
description: hook_output runs the session hook against cli/ without hiding the host's global and system git config.
status: open
timestamp: 2026-11-27T01:50:02Z
---

## 0157. The session-context tests read the host's git configuration

## Context

`hook_output` in `cli/tests/session_context_line.rs` runs `skills/living-docs/hooks/session-context.sh` with `CLAUDE_PROJECT_DIR` set to `cli/` and without `GIT_CONFIG_GLOBAL=/dev/null` or `GIT_CONFIG_NOSYSTEM=1`. Without plugin mode the script runs `git config core.hooksPath` when the root has `.githooks/`. It is harmless today because `cli/.githooks` does not exist, but the tests read the host's git configuration, the failure class issue 0115 fixed for the pre-commit tests. Found in review of issue 0156.

## Decision

- `hook_output` builds a temporary git project and runs the script there with `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`, as the plugin-mode tests do (session, 2026-11-27). Not taken: keeping `cli/` as the project root.

## Acceptance

- `hook_output` runs the script in a temporary project, with the host's global and system git configuration hidden, and the existing assertions stay green.

## Scope

In: `cli/tests/session_context_line.rs`. Out: the script itself.
