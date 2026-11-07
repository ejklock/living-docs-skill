---
type: Issue
title: Project guide is AGENTS.md with CLAUDE.md a symlink to it
description: Make the skill create the project guide as AGENTS.md with CLAUDE.md a symlink, so every harness reads one guide.
status: closed
timestamp: 2026-11-07T14:09:51Z
---

## 0106. Project guide is AGENTS.md with CLAUDE.md a symlink to it

### Scope

The skill tells an adopting project to create its guide as "`CLAUDE.md` or equivalent" (`skills/living-docs/rules/procedure.md`, greenfield step 1), and the hard-rules template and the document map in `doc-trail.md` name `CLAUDE.md` as the guide. Pi, Codex and OpenCode read `AGENTS.md`; Claude Code reads `CLAUDE.md`. A project that follows the skill today gives only Claude Code its guide.

### Decision

- The guide is `AGENTS.md` at the project root, and `CLAUDE.md` is a symlink to it committed to git, so every harness reads one file.
- The rule stays skill text. No verb writes the guide: ADR 0060 fixes the verb set, and the guide is prose the author writes.
- Brownfield adoption moves an existing `CLAUDE.md` to `AGENTS.md` with `git mv` and adds the symlink. When both files exist with different content, the agent asks the owner how to merge them.
- The template keeps its file name `templates/claude-hard-rules.md`, which the ADRs cite as history; only its text points at `AGENTS.md`.

### Acceptance

- `procedure.md` greenfield step 1 creates `AGENTS.md` and the `CLAUDE.md` symlink; brownfield step 1 migrates an existing guide as above.
- `doc-trail.md` names the guide as `AGENTS.md` with `CLAUDE.md` a symlink to it, and the template tells the reader to paste into `AGENTS.md`.
- `git grep "CLAUDE.md\` or equivalent"` finds nothing under `skills/`, and `living-docs guide procedure --plain` from a fresh build shows the new step.
- `living-docs check` passes over `docs/`.
