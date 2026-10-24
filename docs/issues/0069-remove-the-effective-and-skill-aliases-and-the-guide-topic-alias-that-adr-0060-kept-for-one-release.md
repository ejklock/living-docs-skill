---
type: Issue
title: Remove the effective and skill aliases and the guide --topic alias that ADR 0060 kept for one release
description: The hidden aliases ADR 0060 scheduled for removal after v0.18.0 still run in v0.23.0; they leave the CLI so the surface matches the help
status: open
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
