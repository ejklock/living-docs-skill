---
type: Issue
title: The changed-only pre-commit tests inherit the host's git config and environment
description: Isolate the git helper of the changed-only pre-commit tests from the host's HOME, git config and LIVING_DOCS variables, as the binary-order tests already are.
status: closed
timestamp: 2026-11-12T18:19:45Z
---

## 0115. The changed-only pre-commit tests inherit the host's git config and environment

The `git` helper in `cli/tests/pre_commit_changed_only.rs` (`Command::new("git")` near line 30) overrides `PATH` and the author and committer identity but inherits everything else from the developer's machine: `HOME`, the system and global git config, and every `LIVING_DOCS_*` and `GIT_*` variable. A host with `commit.gpgsign=true`, a global `core.hooksPath`, or `LIVING_DOCS_BUNDLE` set in the shell fails these tests for reasons unrelated to the hook. `cli/tests/pre_commit_binary.rs` › `isolated_git` already has the isolation this file lacks.

### Scope

- The helper clears the inherited environment and sets only what the test needs: `HOME` at the scratch root, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=/dev/null`, `PATH` and the identity; the opt-in variable the test drives is set explicitly per case.
- Kept: every assertion and test name.

### Acceptance

- `cargo test -p living-docs --test pre_commit_changed_only` passes plainly, with `LIVING_DOCS_BUNDLE=elsewhere` set in the caller's shell, and with a `HOME` whose `.gitconfig` sets `commit.gpgsign=true` and `gpg.program=/nonexistent`.
