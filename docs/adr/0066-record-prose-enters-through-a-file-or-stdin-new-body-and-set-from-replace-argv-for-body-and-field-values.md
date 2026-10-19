---
type: ADR
title: "Record prose enters through a file or stdin: new --body and set --from replace argv for body and field values"
description: new gains --body PATH|- to fill the scaffolded body from a file or stdin, and set gains --from PATH|- to read any field value the same way, so a model never passes prose through shell quoting nor writes a whole record by hand.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-19T13:13:52Z
---

# 0066. Record prose enters through a file or stdin: new --body and set --from replace argv for body and field values

## Context

The authoring contract since [ADR 0019](/adr/0019-hand-written-record-frontmatter-is-blocked-at-write-time-detected-by-check-and-taught-at-point-of-use.md) is: the CLI scaffolds the numbered record, the author writes the body. The usage analysis over the owner's agent transcripts (8,376 CLI calls, September and October 2026) shows how the body actually arrives. Agents wrote a whole record with a file-writing tool 478 times, 90 of them in a session with no `new` call at all, which hand-rolls the frontmatter the CLI owns and, as recorded in the owner's memory, loses the timestamp `set` cannot restore. `set description` ran 336 times with an average argument of 290 characters passed through shell quoting; 23 of the recorded `status` values carry a stray `;` or `|` from a broken quote. Closing an issue appends an Outcome section, which agents did with `printf >>`, a pattern the owner's Bash hooks block (132 blocked `check` calls were redirects of the same kind).

The language model's natural unit is the whole body or a whole section. argv is hostile to prose: quotes, backticks, dollar signs and newlines all need escaping, and a heredoc is a redirect. Every other deterministic authoring tool that takes prose (git commit `-F`, gh `--body-file`) reads it from a file or stdin.

## Decision

We will let record prose enter through a file or stdin, never through argv, with two additive options on the existing verbs.

- **`new <type> "<title>" --body PATH|-`** replaces the scaffolded body, everything below the heading line, with the file's content. Number, frontmatter and heading stay CLI-owned. The content is taken verbatim, line endings normalized to LF and one trailing newline. A body that opens with a frontmatter fence (`---` on its first line) is refused with exit 2 naming the body-only rule; an empty body is refused with exit 2.
- **`set <ref> <key> [VALUE] --from PATH|-`** reads the value from the file when VALUE is absent, for every settable key. VALUE together with `--from` is a usage error (exit 2). The value is trimmed of surrounding whitespace and validated exactly as an argv value is.
- **`set <ref> section "<heading>" --from PATH|-`** is a new key: it replaces the content under the first body section whose heading matches, case-insensitive, by the same matcher the section schema of [ADR 0064](/adr/0064-each-doc-type-declares-its-sections-in-the-registry-required-or-optional-contract-or-detail-tier-so-check-enforces-the-required-ones-on-live-records-and-read-discloses-by-tier.md) uses, keeping the heading line; when no section matches it appends the heading and the content at the end of the body, at the heading level the record's other sections use (`##` when it has none). The tool places text where told and never writes prose: the determinism boundary of the project holds.
- **`-` is stdin** in both verbs. `-` with a terminal on stdin is a usage error with a hint, so a model that forgets the pipe is not left hanging.

Rejected alternatives:

- **Teach heredocs.** The quoting hazards stay (`$` and backticks expand inside an unquoted heredoc), and a heredoc is a redirect that the owner's and others' Bash hooks block.
- **`--body "text"` in argv.** It puts the prose back into the shell.
- **Keep the Edit tool as the only body path.** 478 whole-record writes say the whole body is the unit a model reaches for; the tool should accept that unit instead of punishing it.
- **JSON on stdin.** More structure than the content needs; a model that has written Markdown already has the file.
- **A `close` verb that sets status and writes the outcome.** It encodes one workflow into the surface; `set status` plus `set section Outcome` composes the same result from existing verbs and serves any other section.

## Consequences

**Easier / gained:**
- A record is born complete in one call with CLI-owned frontmatter; a description or a title arrives without quoting; closing an issue with its Outcome is two calls that never touch the shell's quoting rules.
- A harness that cannot write files but can pipe gains a body path.

**Harder / accepted trade-offs:**
- `set`'s VALUE becomes optional in the grammar; help shows both shapes and the error for a missing value names `--from`.
- `section` is the first `set` key that reaches into the body; the body stays author-owned and the tool only replaces the text under one heading.

**Follow-ups:**
- [Issue 0059](/issues/0059-new-body-and-set-from-read-prose-from-a-file-or-stdin.md) implements the two options and the `section` key.
- The procedure and issue-workflow topics and the session hook show `--body` and `--from` as the way prose arrives.

## Verification

**Implementation impact:** `cli/src/args.rs`, `cli/src/args/help.rs`, `cli/src/commands/new.rs`, `cli/src/commands/set.rs`, `living-docs-core/src/commands/new.rs`, `living-docs-core/src/commands/set.rs`, `living-docs-core/src/sections.rs`, `skills/living-docs/rules/procedure.md`, `issue-workflow.md`.

**Verification criteria:**
- `new issue "T" --body f` yields a record whose text below the heading equals `f` and whose frontmatter equals the one `new` writes without `--body`; `check` passes when `f` carries the required sections.
- `set NNNN description --from f` writes the same frontmatter as `set NNNN description "<content of f>"`.
- `set NNNN section Outcome --from f` replaces an existing Outcome and appends one when absent, at the record's heading level.
- `--from -` on a terminal, a body file opening with `---`, an empty file and VALUE together with `--from` each exit 2 with a message naming the rule.
- Integration tests in `cli/tests` cover each case through the binary.
