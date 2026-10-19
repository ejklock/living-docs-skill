---
type: Issue
title: The help documents every argument, vocabulary and reference form, and prints minified JSON off a TTY
description: "Implements ADR 0068: JSON help derived from the clap model off a TTY, every argument described, status vocabularies and set keys from the registry, guide topics and check kinds listed, a tree-walk completeness test"
status: closed
timestamp: 2026-10-19T17:40:25Z
---

## 0062. The help documents every argument, vocabulary and reference form, and prints minified JSON off a TTY

Implements ADR 0068. The help answers, in one call, what an agent needs to run a verb, and obeys the output contract: text on a TTY or `--plain`, minified JSON off a TTY or with `--json`.

### Scope

- JSON help: `--help`, `-h`, `help <verb>` and `--version` print one minified JSON line when stdout is not a TTY or `--json` is given, derived from the clap command tree plus the structured examples and exit codes; the root document carries every verb in full. `--plain` or a TTY keeps clap's text.
- Every positional and option carries help text: `TITLE`, `REFERENCE`, `KEY`, `VALUE`, `OLD`, `NEW`, `PATHS` on `check` and `fmt`, `fmt --check`, `guide TOPIC_OR_SKILL`, `install skills --harness`.
- Registry-rendered vocabularies: `set KEY` lists `status`, `description`, `owner`, `title`, `section` and the status vocabulary per type; `REFERENCE`, `OLD` and `NEW` state the three forms (`NNNN`, `TYPE/NNNN` with the type token or its directory name, a record path ending in `.md`); `guide` lists the embedded topics; `check` lists the kinds it reports with one line each and says violations exit 1 while advisories do not.
- `new --owner` says the value is seeded from git config `user.name`, then `user.email`, on types that require an owner; `--docs-dir` describes the bundle root for the caller; help strings cite no ADR, issue or test.
- README's CLI section states the help modes; the session hook keeps pointing at `--help`.

### Decision

- The JSON help is built from the clap `Command` model at run time, not from a second hand-written table; option not taken: a static JSON document.
- Completeness is enforced by a test that walks the command tree and fails on any argument without help; option not taken: a review checklist.
- The check kinds live in one list in core that the check modules use, so the help and the report share the names; option not taken: a hand-copied list in the CLI.

### Acceptance

- `living-docs --help | cat` prints one line of valid JSON whose commands include the ten verbs and `completions`, each with args, options, examples and exit codes; `living-docs set --help --json` prints one line naming `REFERENCE`, `KEY`, `--from`; `living-docs --help --plain | cat` and `living-docs --help` on a TTY print the text help; `living-docs --version | cat` prints JSON with the version.
- The tree-walk test passes and fails when an argument's help is removed.
- `set --help` text names the five keys and, for every type with a vocabulary, its statuses; `guide --help` names every topic `guide --list` prints; `check --help` names every kind that appears in `check --json` over the test fixtures.
- No help string contains `ADR `, `issue ` or `test`.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. CLI: the help interception and JSON rendering from the clap model; the output-mode decision for help; tests red first through the binary.
2. CLI: every missing help string, the registry-rendered vocabularies, the reference forms; the tree-walk completeness test.
3. Core: the check kind list and its use by the modules; `check --help` renders it.
4. README and the help tests.

### Outcome

Delivered 2026-10-19 on main after v0.22.0, uncommitted until the owner ships it.

- `--help`, `-h`, `help <verb>` and `--version` print one minified JSON line when stdout is not a TTY or `--json` is given, derived from the clap command tree plus the structured examples and exit-code table (`cli/src/args/help_json.rs`); a TTY, `--plain` or `--color=always` keeps clap's text. The root document carries the ten verbs and `completions` in their four groups with args, options, nested subcommands, examples and exit codes; a verb document adds the global options; `--version` prints `{"name","version"}`. An option that takes no value renders `value` and `default` as null. Hidden aliases (`effective`, `skill`, guide `--topic`) stay out of the document.
- Every positional and option of every verb and subverb carries help; `set KEY` lists the five keys and the status vocabulary per type rendered from the type registry, `REFERENCE`/`OLD`/`NEW` state the three reference forms, `new --owner` states the git-config seeding, `guide` lists the embedded topics from the corpus, `check` lists the advisory kinds from the new `living-docs-core::check::kinds` list that the check modules now use. Help strings and touched docblocks cite no ADR or issue.
- Tests: `cli/tests/help_json.rs` (15), `cli/tests/help_json_flags.rs` (5), `cli/tests/help_complete.rs` (8, tree walk over the JSON root, vocabularies derived from the registry, guide topics compared with `guide --list`, kinds compared with the list, no-citation scan), existing `cli/tests/help.rs` kept (46). Workspace: 792 passed, 0 failed. `cargo fmt`, clippy with `-D warnings -W clippy::too_many_lines`, the file-size ratchet, the allow inventory, `make check`, `living-docs check` over `docs` (148 records) and `se-gates check` pass. Review: round 1 requested changes (flags rendered as value-taking; two-token `--color always` untested); round 2 fixed both and was approved.

Deviations from the acceptance as written: the no-citation scan looks for `ADR `, `issue 0`, `tests can` and `temp tree` rather than the bare word `test`, which also matches ordinary words; `check --help` is compared with the kinds list the modules use, not with `check --json` over fixtures, because only advisories carry a kind and violations do not; help on a missing required argument prints the JSON document off a TTY but keeps exit 2, a usage error. Left for a later item: usage errors as JSON on stderr (deferred by ADR 0068); `install --help skills` renders the `skills` document while clap's text renders `install`'s; older record citations remain in `living-docs-core/src/check/mod.rs` docblocks outside the touched one.
