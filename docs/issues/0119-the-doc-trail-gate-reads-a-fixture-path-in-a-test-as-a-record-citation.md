---
type: Issue
title: The doc-trail gate reads a fixture path in a test as a record citation
description: Settle the doc-trail false positive on fixture paths in tests in the gate configuration, then restore the plain literals the 0114 change disguised.
status: open
timestamp: 2026-11-13T01:00:30Z
---

## 0119. The doc-trail gate reads a fixture path in a test as a record citation

The `doc-trail` check of `se-gates check` reads a record path written as a string literal in test code (for example `"docs/adr/0002-clean.md"`, a fixture inside a scratch repository) as a provenance citation in code-facing text, and fails the change. To pass, the issue 0114 change replaced those literals in `cli/tests/pre_commit_changed_only.rs` with an `adr_path` helper that builds the same path with `format!`, which bends the test to the gate instead of settling the gate.

### Scope

- Settle the false positive in the gate's configuration (a `se-gates.config.json` in this repository, or the gate's own rule for test fixtures), so a fixture path in a test is not a citation.
- Restore the plain literals in `cli/tests/pre_commit_changed_only.rs` once the gate accepts them.

### Acceptance

- `se-gates check` passes with `"docs/adr/0002-clean.md"` written as a literal in a test under `cli/tests/`.
- A record citation in a production docblock still fails `doc-trail`.
