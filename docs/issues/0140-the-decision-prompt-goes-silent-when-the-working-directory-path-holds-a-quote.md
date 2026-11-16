---
type: Issue
title: The decision prompt goes silent when the working directory path holds a quote
description: The cwd pattern stops at an escaped quote, so git -C fails and the hook never nudges
status: closed
timestamp: 2026-11-16T04:11:55Z
---

## 0140. The decision prompt goes silent when the working directory path holds a quote

### Scope

The decision-prompt hook reads the payload's `cwd` with the pattern `"cwd"[[:space:]]*:[[:space:]]*"([^"]+)"` (skills/living-docs/hooks/decision-prompt.sh about line 30), which stops at an escaped quote. When the working directory's path contains a `"`, `git -C` then fails and the hook stays silent. Found by the review of issue 0138 slice S2; the file_path parse gained the JSON unescape there, but cwd did not.

### Decision

- Parse cwd with the same escape-aware reader as file_path.

### Acceptance

- With a cwd that holds a quote, a code Edit nudges once and a bundle Edit stays silent; a test proves it.

### Outcome

- The decision-prompt hook reads `cwd` and `file_path` through one escape-aware reader, so a working directory holding a quote or backslash still nudges. Proof: `cli/tests/decision_prompt_cwd.rs`, which starts the hook outside the repository so only the payload cwd can reach it.
