---
type: Issue
title: new --body and set --from read prose from a file or stdin
description: "Implements the ADR that moves prose out of argv: new --body PATH|- replaces the scaffolded body, set --from PATH|- reads the value for any key, and set gains the section key to replace or append one body section."
status: closed
timestamp: 2026-10-19T13:13:52Z
---

## 0059. new --body and set --from read prose from a file or stdin

Implements [ADR 0066](/adr/0066-record-prose-enters-through-a-file-or-stdin-new-body-and-set-from-replace-argv-for-body-and-field-values.md). Prose reaches a record through a file or stdin: `new --body` fills the scaffolded body, `set --from` reads any field value, and the `section` key replaces or appends one body section.

### Scope

- `new <type> "<title>" --body PATH|-`: replaces everything below the heading with the file's content, LF-normalized, one trailing newline. Refused with exit 2 when the content opens with a `---` fence or is empty.
- `set <ref> <key> [VALUE] --from PATH|-`: reads the value from the file when VALUE is absent, trimmed, validated as an argv value is. VALUE with `--from`, and neither, each exit 2 naming the other shape.
- `set <ref> section "<heading>" --from PATH|-`: replaces the content under the first section whose heading matches case-insensitively, keeping the heading line; appends the heading and content at the record's heading level when no section matches (`##` when the record has no section).
- `-` reads stdin; `-` with a terminal on stdin exits 2 with a hint.
- Help for `new` and `set` shows the new shapes in their Examples.
- The procedure and issue-workflow topics and the session hook show `--body` and `--from`.

### Decision

- The section matcher is the one the section schema already uses, so a heading `check` recognizes is a heading `set section` finds; option not taken: an exact-match heading.
- Section content replacement runs from the heading line to the next heading of the same or higher level, so a sub-heading inside the section is replaced with it; option not taken: stopping at any heading.

### Acceptance

- `new issue "T" --body f` writes a record whose text below the heading equals `f`; its frontmatter equals the one written without `--body`; `check` passes when `f` has Scope and Acceptance and fails naming the missing section otherwise.
- `new issue "T" --body f` where `f` opens with `---`, and where `f` is empty, each exit 2 and write nothing.
- `set NNNN description --from f` produces the same frontmatter as the argv form; `set NNNN status --from f` with `closed` in `f` closes the record and with `bogus` fails as the argv form does.
- `set NNNN section Outcome --from f` replaces an existing Outcome including a sub-heading under it, and appends `### Outcome` to an issue that has none.
- `set NNNN description "x" --from f` and `set NNNN description` each exit 2.
- `echo closed | living-docs set NNNN status --from -` closes the record; `--from -` with a terminal on stdin exits 2.
- The full suite and `make check` pass.

### Plan

1. Core: a body-content reader shared by both verbs (file or stdin, LF-normalization, the frontmatter-fence and empty refusals); `new` body replacement; unit tests red first.
2. Core: section replace-or-append in the sections module; `set` dispatch for `section` and `--from`.
3. CLI: the clap grammar (optional VALUE, `--from`, `--body`), help examples, integration tests through the binary; docs topics and the session hook.

### Outcome

Delivered, uncommitted, in the working tree: `new --body PATH|-` replaces everything below the heading with the given text; `set <ref> <key> --from PATH|-` reads any field value from a file or stdin; `set <ref> section "<heading>" --from PATH|-` replaces the text under a heading up to the next heading of the same or higher level, or appends the section at the level the record uses. Every read of a file or stdin lives in `cli/src/commands/prose.rs`; core receives text. Refusals exit 2: a body opening with `---`, an empty body, stdin on a terminal, VALUE together with `--from`, neither. Proof: `cargo test -p living-docs --test new_body` (9 passed), `--test set_from` (15 passed), `cargo test --workspace` green (749), `se-gates check` pass. This Outcome was written with `set issue/0059 section Outcome --from -`.
