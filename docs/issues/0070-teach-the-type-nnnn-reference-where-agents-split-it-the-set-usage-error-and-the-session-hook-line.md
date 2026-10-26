---
type: Issue
title: "Teach the TYPE/NNNN reference where agents split it: the set usage error and the session hook line"
description: set adr 0001 status Accepted fails with a bare clap error; the error names the TYPE/NNNN form, and the session hook line shows it with the number key and the merge repair
status: closed
timestamp: 2026-10-24T10:17:33Z
---

## 0070. Teach the TYPE/NNNN reference where agents split it: the set usage error and the session hook line

Found by the usage audit: `set --help` was read 148 times and `new --help` 195 times, and agents write `set adr 0001 status Accepted`, which fails with "unexpected argument 'Accepted' found" and no hint. The session hook line teaches `set <NNNN>`, which is ambiguous as soon as two types share a number.

### Scope

- When `set` receives a type token followed by a number as two arguments, the usage error says to write them as one reference and shows the corrected command, still exiting 2.
- The session hook line (`skills/living-docs/hooks/session-context.sh`) shows `TYPE/NNNN`, the `number` key of `set`, and that `living-docs index` repairs an index after a merge.

### Decision

- The CLI does not start accepting the split form; option not taken: a second reference grammar, which the help contract of [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) would have to document.

### Acceptance

- `living-docs set adr 0001 status Accepted` exits 2 and its stderr contains `set adr/0001 status Accepted`.
- The hook line names `TYPE/NNNN`, `number` and the merge repair, and its test asserts each.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. CLI: the split-reference hint; tests through the binary.
2. The hook line and its test, after the number key and the union merge land.

### Outcome

Delivered in 799d1c7 and 201143a. `set` given a type token and a number as two arguments still exits 2, and stderr adds `hint: write the reference as one token: living-docs set adr/0001 status Accepted` (cli/src/split_reference.rs; cli/tests/set_split_reference.rs); the CLI does not accept the split form. The session hook line (skills/living-docs/hooks/session-context.sh) names `TYPE/NNNN`, `set <TYPE/NNNN> number next` and that `living-docs index` repairs an index after a merge, each asserted in cli/tests/session_context_line.rs. Verified 2026-10-26: `set adr 0001 status Accepted` exits 2 with the hint; session_context_line 10/0.
