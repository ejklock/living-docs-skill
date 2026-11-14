---
type: ADR
title: The CLI groups its operations under write, read and manage as nested subcommands, with check kept top-level
description: Group the CLI's operations under write, read and manage as nested subcommands, keep check top-level, and remove the old verbs in 0.28.0 with no aliases.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-09T12:03:06Z
---

# 0076. The CLI groups its operations under write, read and manage as nested subcommands, with check kept top-level

## Context

[ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) fixed the CLI at ten top-level verbs plus a `completions` utility: `new`, `set`, `supersede`, `index`, `check`, `fmt`, `read`, `guide`, `install`, `uninstall`. A human meeting the tool has to learn eleven names that answer three questions: how do I write a record, how do I read what is in force, how do I maintain the bundle and its installation. Two of the names exist only to repair a bundle (`index`, `fmt`), and two read different things (`read` the records, `guide` the rules).

The owner wants those three questions to be the CLI's three verbs, write, read and manage, for human use.

The form matters because clap subcommands already carry per-operation arguments, validation and help. A flag discriminant (`write --kind new --type adr --title ...`) would turn each verb into the union of its operations' flags and move a missing argument from parse time to run time. Nested subcommands keep the three-verb model without that cost.

## Decision

We will group the CLI's operations under three verbs, with the operation as a nested subcommand, and keep `check` as a top-level verb because it is the gate every hook and CI job calls:

| Today | From this decision |
|---|---|
| `new <type> <title>` | `write new <type> <title>` |
| `set <ref> <key> ...` | `write set <ref> <key> ...` |
| `supersede <old> <new>` | `write supersede <old> <new>` |
| `read [--topic ...]` | `read records [--topic ...]` |
| `guide [topic]` | `read guide [topic]` |
| `index` | `manage index` |
| `fmt [--check]` | `manage fmt [--check]` |
| `install skills\|hooks` | `manage install skills\|hooks` |
| `uninstall hooks` | `manage uninstall hooks` |
| `completions <shell>` | `manage completions <shell>` |
| `check` | `check` |

- **Arguments, flags, outputs and exit codes do not change.** Each operation keeps its current arguments and flags, its JSON and plain output, and the exit codes ADR 0060 documented; only the path to it changes. The global flags (`--docs-dir`, `--json`, `--plain`, `--color`, `--quiet`) stay global.
- **A verb with no operation prints its own help and exits 2**, listing its operations, so `living-docs write` teaches the next word.
- **A direct break, no aliases.** The old top-level names are removed in the same release; there is no deprecation window. The release is a minor version (0.28.0; 0.27.0 shipped before the regroup landed), not 1.0.0, because the project is pre-1.0 and the owner is not cutting 1.0 yet.
- **Everything that teaches or calls a command moves in the same release.** The binary's own help, hints and error messages; the session-teaching and pre-commit hooks; the skill corpus served by `read guide`; the README, CONTRIBUTING and the project guide; the Makefile, CI and install scripts.

This replaces the verb list of ADR 0060. ADR 0060's output contract (one output mode per stream, JSON off a TTY), its exit codes and its help-for-an-agent rule stay in force.

Rejected alternatives:

- **Keep the ten verbs.** No break, but a human still learns eleven names for three questions.
- **A `--kind` flag discriminant** (`write --kind new ...`). One flat parser per verb, but each verb's help becomes the union of its operations' flags and clap can no longer reject a missing argument per operation.
- **`check` inside `read` (`read check`).** One verb fewer, but every pre-commit hook and CI job that calls `living-docs check` breaks for no gain to the human reader.
- **Old names as hidden aliases for one release.** Softer for consumers, but the owner chose a direct break; with no alias, a stale script fails loudly at its first run instead of drifting silently.
- **`check --fix` absorbing `index` and `fmt`.** Fewer verbs without regrouping, but it leaves the other seven names and makes the gate write.

## Consequences

**Easier / gained:**
- A human learns four words, `write`, `read`, `manage` and `check`, and discovers each operation from that verb's help.
**Harder / accepted trade-offs:**
- Every command in consumer scripts, installed hooks and skills breaks at 0.28.0. A consumer must upgrade the binary and rerun `living-docs manage install hooks` and `living-docs manage install skills` together.
- Common commands get one word longer (`write new` for `new`).
- About 97 files in this repository cite the current names, most of them integration tests; the change is mechanical but wide.

**Follow-ups:**
- [Issue 0109](/issues/0109-regroup-the-cli-under-write-read-and-manage-with-check-top-level-and-move-every-surface-that-teaches-a-command.md) carries the change, sliced so the binary, its tests and every teaching surface land in one release.

## Verification

**Implementation impact:** `cli/src/args/` and `cli/src/main.rs` (the clap tree and dispatch), `cli/src/commands/` (hints that name a command), `living-docs-core` messages that name a command, `cli/tests/`, `skills/*/rules/` and `skills/living-docs/hooks/`, `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `Makefile`, `install.sh`, `npm/`.

**Verification criteria:**
- `living-docs --help` lists exactly `write`, `read`, `manage`, `check` and `help`.
- Each row of the Decision table runs through its new path with the same output and exit code as the old command on the same bundle.
- Each old top-level name exits 2 as an unknown subcommand.
- `git grep` finds no old-form command (`living-docs new`, `living-docs set`, `living-docs supersede`, `living-docs index`, `living-docs fmt`, `living-docs guide`, `living-docs install`, `living-docs uninstall`, `living-docs completions`, and `living-docs read` not followed by `records` or `guide`) outside `docs/adr`, `docs/issues`, `docs/research` and `CHANGELOG`-style history.
- The installed pre-commit hook and the session-teaching hook run green against this repository after `manage install hooks`.
