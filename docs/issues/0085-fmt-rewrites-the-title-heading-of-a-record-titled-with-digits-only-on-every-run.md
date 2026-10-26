---
type: Issue
title: fmt rewrites the title heading of a record titled with digits only on every run
description: <One sentence — the change and its motivation.>
status: closed
timestamp: 2026-10-25T02:00:31Z
---

## 0085. fmt rewrites the title heading of a record titled with digits only on every run

Found by the review of the canonical supersedes work (2026-10-24): a record whose title is the text `0001` gets its title heading rewritten by a second `fmt` to `# 0001. 0001`, so `fmt` is not idempotent on it. The same happens on the base before that work.

### Scope

- `fmt` aligns the title heading of a record whose title is all digits once, and a second `fmt` changes nothing.

### Acceptance

- A heading counts as numbered only when it opens with four digits followed by a dot (`# 0001. X`, `# 0001.`); a bare `# 0001` is unnumbered text, never gains a number, and is aligned to the title like any unnumbered heading.
- An ADR numbered 0003 titled `0001` with the heading `# Something`: after one `fmt` the heading reads `# 0001`, a second `fmt` is byte-identical, and `check` is clean. The same holds when the title equals the record's own number, and for a CRLF record, which keeps CRLF.
- A heading already numbered (`# 0003. 0001`) and a title that starts with digits and continues with words (`# 0003. 2024 roadmap`) keep today's heading.
- Behavior the base had stays: a bare `# 2024` that disagrees with the title `Roadmap` raises the HEADING advisory and `fmt` aligns it; `set number` on a record whose heading is `# 0003` leaves `# 0003`.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the title-heading match `fmt` and `check` share, tests red first.

### Outcome

`fmt` is now idempotent on a record titled with digits only. The cause was `heading_number` (living-docs-core/src/commands/set/retitle.rs) reading a bare `# 0001` as numbered, so a heading written as the title alone was re-read as a number and gained a second one. A heading is numbered only when four digits are followed by a dot; a bare `# NNNN` is text. This rule replaces the first Acceptance wording, which expected `# 0003. 0001`: `fmt` never adds a number to an unnumbered heading. It also restores two base behaviors that a first attempt had changed: the HEADING advisory on a bare `# 2024` and `set number` leaving `# 0003` alone. Tests: cli/tests/fmt_numeric_title.rs (9 cases, each fails when the dot rule is reverted). Suite 1145/0 in review; review approved with two nits: the line-ending trim in `heading_number` no longer changes any result, and the CRLF test does not assert the line endings it names (core unit tests do).
