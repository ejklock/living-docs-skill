---
type: Issue
title: Re-measure the MCP check task threshold on the CI runner
description: Measure on a GitHub-hosted runner the record count where check takes 2 s and update the task threshold that issue 0156 measured on the development host.
status: open
timestamp: 2026-11-27T15:52:15Z
---

## 0162. Re-measure the MCP check task threshold on the CI runner

## Context

ADR 0112 fixes the record count from which the MCP `check` runs as a task where `check` takes 2 s on the CI runner. Issue 0156 measured it on the development host instead (AMD Ryzen 7 5700G, 2670 in-force records, `scripts/bench/check-task-threshold.md`), because CI runs heavy jobs only on manual dispatch and a benchmark job was its own scope. A GitHub-hosted runner is slower than that host, so the threshold there is probably lower; this is an inference, not measured.

## Decision

- A separate workflow, `bench.yml`, runs only on `workflow_dispatch` (heavy jobs stay manual), builds the release binary, runs `scripts/bench/check-task-threshold.sh` and uploads its output as an artifact; after it reaches `main`, the session dispatches it, reads the artifact and updates the constant and the measurement file (session, 2026-11-27). Not taken: a step in the `check` job.

## Acceptance

- A manually dispatched workflow job runs `scripts/bench/check-task-threshold.sh` on the CI runner and uploads its output.
- `CHECK_TASK_THRESHOLD` in `mcp/src/tasks.rs` and `scripts/bench/check-task-threshold.md` are updated from that run, or the measurement shows the host value is kept and says why.

## Scope

In: a `workflow_dispatch` job, the threshold constant and its measurement file. Out: the task mechanism.
