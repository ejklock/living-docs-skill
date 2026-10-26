---
type: ADR
title: "Records keep their own line endings: CRLF is read and written as CRLF, and a mixed-ending record fails check"
description: A CRLF record is read and written as CRLF through one conversion at the store edge; a record with mixed endings is refused before any write by set, supersede and fmt and fails check
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-24T19:31:39Z
---

# 0072. Records keep their own line endings: CRLF is read and written as CRLF, and a mixed-ending record fails check

## Context

The verbs detect a record's frontmatter by a bare `---` line and split it with `lines()`. A record saved with CRLF line endings opens with `---\r`, so the tools misread it:

- `fmt` adds a second frontmatter block to it (issue 0072).
- `check` reports a false "frontmatter has no non-empty 'type'" for it.
- `set` and `supersede` either write a file with mixed endings or, after the read-back guard on every write, refuse it with a message that does not name the cause.

A CRLF record comes from a Windows editor or a checkout with `core.autocrlf`. A record with mixed endings comes from a hand edit of a CRLF file in an LF editor, or the reverse.

Git solves the same problem with `core.autocrlf` and `safecrlf`, and editors with editorconfig's `end_of_line`: the file is normalized to LF on input and converted back on output, and only the files being converted are judged.

## Decision

We will keep each record's own line endings. A record whose every line break is CRLF is read and written as CRLF, and a record with mixed endings is refused by the verbs that write it and fails `check`.

- **One conversion at the edges.** A store view in core reads a CRLF record as LF and writes it back with CRLF on every line. The parsers stay LF-only. An accepted write on a CRLF record equals the same write on its LF twin, with every `\n` as `\r\n`.
- **Verbs.** `set` (every key, `section` and `title` included), `supersede` and `fmt` read and write through that view, and so does `check`. The type-index refresh, which reads and writes outside the store, converts at its own edge, so a CRLF index stays CRLF.
- **Mixed endings are refused before any write.** `set`, `supersede` and `fmt` refuse a record they would draft whose line breaks are both CRLF and bare LF, naming "mixed line endings". The refusal comes before the first write or rename, so every file stays byte-identical. Other files the verb only scans or rewrites by reference (siblings, prose, reserved `index.md` and `log.md`) are read and written exactly as before, mixed bytes included.
- **Mixed endings fail `check`.** `check` judges a mixed record on its LF form, so it reports no false frontmatter finding. It also reports a violation naming the record and its mixed line endings, so the pre-commit doc-gate blocks it until the author picks one ending.
- **`new` writes LF.** Templates and new records keep LF.

Rejected alternatives:

- **Refuse every CRLF record.** It is simple, but it locks Windows authors out of every write verb for a file their editor saved correctly.
- **CRLF-aware parsing at each `---` and `lines()` site.** It covers the same cases, but the rule is spread over every parser, and each new parser must remember it.
- **Normalize mixed files to one ending.** The tool would have to choose which ending wins, and it would rewrite lines the author did not touch.
- **Strict reads for the whole bundle.** It was tried and reverted: one mixed sibling dropped rows from the regenerated index, and it left `set title` half-done after the rename.

## Consequences

**Easier / gained:**
- CRLF bundles work with every authoring verb, and `fmt` no longer corrupts them.
- A mixed record is caught at commit, not left for the next write verb to refuse.

**Harder / accepted trade-offs:**
- A bundle that already holds a mixed record fails `check` after the upgrade. The repair is to save the file with one ending.
- Code that reads or writes records outside the store must convert at its own edge.

**Follow-ups:**
- None.

## Verification

**Implementation impact:** `living-docs-core/src/line_ending.rs`, `living-docs-core/src/commands/set.rs`, `living-docs-core/src/commands/supersede/mod.rs`, `living-docs-core/src/commands/fmt.rs`, `living-docs-core/src/commands/index.rs`, `living-docs-core/src/check/mod.rs`, a check module for mixed line endings, `living-docs-core/src/check/kinds.rs`, `cli/tests/*`.

**Verification criteria:**
- Each write verb on a CRLF record equals its LF twin with CRLF, byte for byte, with no bare LF in the result.
- `fmt` on a CRLF record with unordered keys leaves one frontmatter block, keys in canonical order and CRLF endings. A second run writes nothing.
- `check` on a CRLF record reports what it reports on its LF twin.
- A mixed record is refused by `set`, `supersede` and `fmt` with every file byte-identical, and `check` exits 1 naming it. A mixed sibling, prose file or reserved index does not change what the verbs do to clean records.
