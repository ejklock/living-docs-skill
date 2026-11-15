---
type: Issue
title: Renumbering renames the record before rewriting references, so a failed rewrite leaves the bundle half-renumbered
description: relocate renames the file and then rewrites in-bundle references; a write that fails afterwards leaves a renamed file, partly rewritten references and a stale index
status: open
timestamp: 2026-11-15T18:19:13Z
---

## 0130. Renumbering renames the record before rewriting references, so a failed rewrite leaves the bundle half-renumbered

### Scope

`living-docs-core/src/commands/set/retitle.rs` › `relocate` renames a record and then rewrites the in-bundle references to it. When a rewrite fails after the rename (seen with non-canonical frontmatter, which `check` already flags), the command exits non-zero but leaves the file renamed, some references rewritten and the index not regenerated, so `check` then reports index drift. Found by the review of issue 0125 slice S3, whose own blocked_by rewrites now draft before the rename; the older reference rewrite in `relocate` keeps the pattern.

### Decision

- Draft every reference rewrite before the rename and abort with no change on disk when any draft fails, as the blocked_by rewrite already does.

### Acceptance

- A renumber or retitle whose reference rewrite would fail exits non-zero with every file byte-identical and no index drift; a test proves it.
