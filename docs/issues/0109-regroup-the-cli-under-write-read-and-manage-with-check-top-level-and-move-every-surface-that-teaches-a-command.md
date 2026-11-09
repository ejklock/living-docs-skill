---
type: Issue
title: Regroup the CLI under write, read and manage with check top-level, and move every surface that teaches a command
description: Regroup the CLI under write, read and manage with check top-level, as ADR 0076 proposes, and move every surface that teaches a command in the same release.
status: open
timestamp: 2026-11-09T12:08:07Z
---

## 0109. Regroup the CLI under write, read and manage with check top-level, and move every surface that teaches a command

## Regroup the CLI under write, read and manage with check top-level, and move every surface that teaches a command

Implements [ADR 0076](/adr/0076-the-cli-groups-its-operations-under-write-read-and-manage-as-nested-subcommands-with-check-kept-top-level.md): the ten verbs and the `completions` utility become operations nested under `write`, `read` and `manage`, `check` stays top-level, and the old top-level names are removed in 0.27.0 with no aliases.

### Scope

- The clap tree and dispatch (`cli/src/args/`, `cli/src/main.rs`), the root and per-verb help, and every hint or error message in `cli/src` and `living-docs-core` that names a command.
- The integration tests in `cli/tests/` and the fixture runners that call the binary (`skills/living-docs/tests/run.sh`, `scripts/tests/`).
- Every surface that teaches a command: `skills/*/rules/`, `skills/*/SKILL.md`, `skills/living-docs/templates/`, `skills/living-docs/hooks/` (session teaching and pre-commit), `skills/living-docs-drift/`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `Makefile`, `install.sh`, `npm/`, `examples/`.
- The CLI-written retired-record callout and index note ("Run `living-docs read` for what is in force") take the new path; existing records are regenerated through the CLI, never hand-edited.
- Kept: every operation's arguments, flags, JSON and plain output and exit codes; record bodies under `docs/adr`, `docs/issues` and `docs/research` that cite old commands as history.

### Decision

- **A bare verb prints its help and exits 2.** `living-docs write`, `read` and `manage` with no operation list their operations; not chosen: defaulting `read` to `read records`, which would make `read` the one verb that behaves differently.
- **Two slices in one release.** The binary with its tests first, then the teaching surfaces; both merge before 0.27.0 is tagged, so no release ships a binary its own guide contradicts.

### Acceptance

- `living-docs --help` lists exactly `write`, `read`, `manage`, `check` and `help`; `write --help` lists `new`, `set`, `supersede`; `read --help` lists `records`, `guide`; `manage --help` lists `index`, `fmt`, `install`, `uninstall`, `completions`.
- Every row of ADR 0076's table runs through its new path with the same stdout, stderr and exit code as the old command produced on the same bundle.
- Each old top-level name (`new`, `set`, `supersede`, `index`, `fmt`, `guide`, `install`, `uninstall`, `completions`) exits 2 as an unknown subcommand, and bare `read` exits 2 with its help.
- Every hint and error the binary prints names the new path (for example a check failure that suggests `living-docs manage fmt`).
- `git grep` finds no old-form command outside `docs/adr`, `docs/issues` and `docs/research`.
- After `living-docs manage install hooks` in this repository, the pre-commit hook and the session-teaching hook run green and teach the new paths.
- `make check` and CI pass; `living-docs check` passes over `docs/`.

### Plan

1. Binary: clap tree, dispatch, help, hints and messages, and every test or fixture runner that calls the binary.
2. Teaching surfaces: rules, templates, SKILL.md files, hooks, README, CONTRIBUTING, CLAUDE.md, Makefile, install.sh, npm, examples, and the CLI-written callouts regenerated over `docs/`.
