---
type: Issue
title: Remove the effective and skill aliases and the guide --topic alias that ADR 0060 kept for one release
description: The hidden aliases ADR 0060 scheduled for removal after v0.18.0 still run in v0.23.0; they leave the CLI so the surface matches the help
status: closed
timestamp: 2026-10-24T10:17:32Z
---

## 0069. Remove the effective and skill aliases and the guide --topic alias that ADR 0060 kept for one release

[ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) kept `effective` and `skill` as hidden aliases for exactly one release after v0.18.0. In v0.23.0 both still run (`living-docs effective --help` and `living-docs skill --help` print the `read` and `guide` documents, verified 2026-10-24), and guide keeps a hidden `--topic` alias. The usage audit counts few calls to them.

### Scope

- Remove the `effective` and `skill` subcommand aliases and the `guide --topic` alias, with their tests and every live document that still names them (retired ADR bodies keep them as history).
- `guide --skill` stays: it is a real option, not an alias.

### Acceptance

- `living-docs effective`, `living-docs skill` and `living-docs guide --topic adr` exit 2 as unknown usage.
- `git grep -n -w -e effective -e 'living-docs skill' -- cli/src skills README.md CLAUDE.md` prints no use of the aliases as commands.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Remove the aliases and their tests; update documents.

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog.

- `cli/src/args.rs`: the `effective` and `skill` subcommand aliases are gone; `living-docs effective` and `living-docs skill` exit 2 through clap's unknown-subcommand error.
- `cli/src/args/sub.rs` and `cli/src/commands/guide.rs`: the hidden `guide --topic` field and its resolver are gone; `guide --topic adr` exits 2 as an unexpected argument. `read`, `read --topic`, `guide <topic>` and `guide --skill` behave as before.
- Tests: `cli/tests/guide.rs` refuses `skill` and `guide --topic` and checks that `guide adr` and `guide --skill living-docs` still print their documents; `cli/tests/read.rs` refuses `effective`; the help and `help --json` absence assertions stay. Each refusal test failed when its alias was planted back, and the kept-skill test failed when `--skill` printed another skill's body.
- No live document named the aliases as commands; retired ADR bodies keep them as history.
- Verification in the main checkout: `cargo test --workspace` 852 passed, 0 failed; `make check` (file-size ratchet and allow-inventory included), clippy with `-D warnings`, `cargo fmt --check`, `living-docs check docs` and `se-gates check` pass. Review approved with one normal finding (the kept-skill test checked only the exit code), fixed and approved on narrow re-review.
