---
type: Issue
title: session-context.sh header comment understates what the session line teaches
description: Align the hook's header comment with the line it emits
status: closed
timestamp: 2026-11-15T16:24:30Z
---

## 0124. session-context.sh header comment understates what the session line teaches

### Scope

The header comment of `skills/living-docs/hooks/session-context.sh` (lines 6-7) says the emitted line states "the body-only rule and the resolved CLI binary". The line has long also taught the write and read verbs, the index repair and, since issue 0144 slice S3, where a mid-work choice is recorded. The comment understates what the hook does. Found by the review of that slice; it predates it.

### Decision

- Rewrite the header comment to say what the line teaches in one sentence, without listing every verb, so it does not drift again.

### Acceptance

- The header comment describes the line as the authoring contract plus the read-before-deciding and decision-recording rules; `make check` stays green.

### Outcome

- The header comment of `session-context.sh` states in one sentence what the line teaches; the emitted line is byte-identical.
