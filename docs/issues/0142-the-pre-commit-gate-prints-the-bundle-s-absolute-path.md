---
type: Issue
title: The pre-commit gate prints the bundle's absolute path
description: When the pre-commit hook runs check, its header shows the full absolute bundle path, which is long and leaks the machine layout into commit output
status: open
timestamp: 2026-11-16T04:49:05Z
---

## 0142. The pre-commit gate prints the bundle's absolute path

### Scope

The QA walkthrough of 2026-11-16 showed the pre-commit doc-gate printing `Living Docs lint — bundle: /Volumes/.../.living-docs/qa/demo/docs`, the absolute path, while a direct `living-docs check` prints `bundle: docs`. The long path wraps in a terminal and shows the machine's directory layout in commit output.

### Decision

- Print the bundle path relative to the repository root in the pre-commit hook, as the direct command does.

### Acceptance

- A commit through the installed hook prints `bundle: docs`; a test proves it.
