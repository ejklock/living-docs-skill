---
type: Issue
title: jscpd ignores tests and markdown so the duplication gate measures production code
description: A root .jscpd.json ignores test code and markdown, so the duplication gate blocks only on production clones instead of on fixtures a change happens to touch
status: closed
timestamp: 2026-09-23T14:00:22Z
---

## 0050. jscpd ignores tests and markdown so the duplication gate measures production code

`se-gates check` runs the `jscpd` duplication check over the whole tree. This repository has no committed baseline ([issue 0046](0046-packaging-is-the-cli-install-sh-bootstraps-the-binary-only-and-the-harness-install-matrix-the-claude-code-plugin-channel-and-the-generated-copilot-copy-leave-the-repo.md) removed it), so the gate blocks every clone whose either side is a changed file. On 2026-09-23 the tree held 69 clones; 52 sat in test code or markdown, where repeated fixtures and prose are expected. Issue 0048 paid that toll twice for a change that did not add any production duplication.

### Scope

- A `.jscpd.json` at the repository root, which `se-gates` passes to `jscpd` as its native config: `minLines: 5` (the gate's default, kept explicit because a native config replaces it) and an ignore list for test code and markdown: `cli/tests/`, `scripts/tests/`, `**/tests.rs`, `**/*_tests.rs`, `**/tests/**`, `living-docs-core/src/test_support.rs`, `**/*.md`.
- The inline `#[cfg(test)]` module of `living-docs-core/src/check/canonical.rs` (about 200 lines, over the ~100-line sibling threshold of CLAUDE.md rule 5) moves to `check/canonical/tests.rs`, so the ignore covers it.
- Kept: no baseline file. The nine production clones that remain are paid down when a change touches them, as issue 0046 decided.

### Decision

Ignore by path rather than commit a baseline. The ignore list names a category (tests, markdown) that stays true as files are added, while a baseline freezes today's clone list and has to be regenerated. The option not taken was `se-gates qg baseline`, which would also reverse issue 0046's removal.

### Acceptance

- `se-gates qg jscpd` reports no clone with a side in test code or markdown.
- `se-gates qg jscpd` still reports a production clone: `fs-store/src/lib.rs` against `living-docs-core/src/commands/next.rs` stays listed, so the ignore did not switch the check off.
- The `canonical` tests keep their count and pass from `check/canonical/tests.rs`.
- `cargo test --workspace`, clippy, fmt and `scripts/check-file-size.sh` stay green.
