---
type: ADR
title: "Help is part of the output contract: complete for every verb, text on a TTY, minified JSON off it or with --json"
description: help, help <verb> and --version print clap's text on a TTY or with --plain and one minified JSON document derived from the command tree off a TTY or with --json; every positional and flag is described, vocabularies render from the registry, and a tree-walk test enforces completeness
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-19T17:40:25Z
---

# 0068. Help is part of the output contract: complete for every verb, text on a TTY, minified JSON off it or with --json

## Context

ADR 0060 made the surface a contract: ten verbs, JSON off a TTY for every data verb, documented exit codes, help with examples. Help itself was left half-way. Measured over 8376 CLI calls in 4304 agent sessions up to 2026-10-19, 7% of all calls are `--help` (`new` 169, `set` 140, `guide` 59, `index` 47), and the same sessions then fail on what the help never said: the status vocabulary of a type, the keys `set` accepts, the forms a reference takes, the topics `guide` serves, the kinds `check` reports. Today `REFERENCE`, `KEY`, `OLD`, `NEW`, `TITLE`, `PATHS` and `fmt --check` print no description at all, `--owner` does not say it is seeded from git config, and several descriptions cite ADR or issue numbers that mean nothing to a caller. Help is also the one output that ignores the output contract: an agent piping `living-docs set --help` gets wrapped terminal text, not the minified JSON every other verb gives a pipe.

## Decision

Help joins the output contract and becomes complete.

- **One mode per stream applies to help and version.** `--help`, `-h`, `help <verb>` and `--version` print clap's text on a TTY or with `--plain`, and one minified JSON document when stdout is not a TTY or `--json` is given. The JSON is derived from the same command tree clap renders, so it cannot drift from the text: name, about, usage, positionals and options (name, value name, required, default, possible values, help), examples, exit codes. The root help carries every verb in full, so one piped `living-docs --help` yields the whole contract; a verb's help carries that verb plus the global options. The document gains fields, never loses or renames them, as every other JSON shape under ADR 0060.
- **Every argument is described, and vocabularies come from the registry.** The type list, the status vocabulary per type and the key list of `set` render from the doc-type registry at runtime; the topics of `guide` render from the embedded corpus; the kinds `check` reports render from one list the check modules themselves use. A positional or flag without help text fails a test that walks the command tree, so completeness is a fitness function, not a review item.
- **Help text is written for the caller.** Descriptions name behavior and accepted values, never an ADR, issue or test concern; `--docs-dir` says what it is for the caller, not for the test suite.

Rejected alternatives:

- **Leave help as terminal text and point agents at `guide`.** Rejected: the metric shows agents call `--help` first and `guide` far less; the text they get wraps, colors nothing, and omits the vocabularies, so they call it again or guess.
- **A hand-written JSON help document.** Rejected: a second description of the surface drifts on the first flag added; deriving from the clap model keeps one source.
- **Usage errors as JSON on stderr in the same change.** Deferred: it touches every verb's error path and the exit-2 texts the skill topics quote; it gets its own record if the metric still shows agents re-parsing errors.

## Consequences

**Easier / gained:**
- One piped `--help` gives an agent the complete, parseable surface, and the text help answers the questions the metric shows agents asking.

**Harder / accepted trade-offs:**
- The JSON help shape becomes a contract to keep.
- Help strings grow; the per-type status table and the kind list add lines to `set --help` and `check --help`.

**Follow-ups:**
- Re-measure the `--help` share and the status and key errors after one release.

## Verification

**Implementation impact:** `cli/src/args.rs`, `cli/src/args/help.rs`, `cli/src/args/sub.rs`, a help-rendering module in the CLI, `living-docs-core/src/check` (the kind list), `cli/tests/help.rs`, `README.md`.

**Verification criteria:**
- `living-docs --help | cat` and `living-docs set --help --json` each print one line of valid JSON naming every non-hidden verb or that verb's arguments; `living-docs --help --plain | cat` prints the text help.
- A test walking the clap command tree finds no positional or option without help text.
- `set --help` names the five keys and the status vocabulary of every type that has one; `guide --help` names every embedded topic; `check --help` names every kind the check modules emit.
