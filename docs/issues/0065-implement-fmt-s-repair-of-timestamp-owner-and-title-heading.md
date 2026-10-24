---
type: Issue
title: Implement fmt's repair of timestamp, owner and title heading
description: "Implements ADR 0071: fmt seeds a missing timestamp from the adding commit, a missing required owner from git config, and aligns the title heading, keeping every other body byte identical"
status: closed
timestamp: 2026-10-24T10:17:28Z
---

## 0065. Implement fmt's repair of timestamp, owner and title heading

Implements [ADR 0071](/adr/0071-fmt-repairs-a-record-s-cli-owned-fields-it-seeds-a-missing-timestamp-and-owner-and-aligns-the-title-heading-leaving-all-other-prose-byte-identical.md).

### Scope

- `fmt` seeds a missing `timestamp` from the author date of the commit that added the file, or the current UTC time when the file was never committed; a present one is never changed.
- `fmt` seeds a missing `owner` on a type that requires one from git config `user.name`, then `user.email`; with neither, it stays absent.
- `fmt` rewrites the record's first heading to its `NNNN.` prefix and the frontmatter title, keeping the heading level; a record without a frontmatter title gets it from the heading text.
- `fmt --check` reports these repairs and writes nothing; `fmt --help`, the procedure and check topics and the README describe them.

### Decision

- Owner seeding reuses the rule `new` applies and the heading rewrite reuses the retitle code; option not taken: copies of either.
- Git is read through the process the CLI already uses for git config; option not taken: a git library dependency.
- A retired record (superseded or deprecated) gets no heading alignment and no title seeding, matching the HEADING advisory, which already skips retired records; option not taken: aligning them, which would rewrite 33 historical headings in this repository.
- The title heading is the first heading outside a fenced block that carries the record's own `NNNN.` prefix from its filename, else the first unfenced level-1 heading (the research template's `# <question>` shape), else none; with none, fmt aligns nothing and seeds no title, and check's HEADING advisory judges the same heading; option not taken: the first heading of any kind, which renamed a section heading such as `## Context`.
- A record with no number in its filename (an architecture view, the constitution) gets no heading alignment from fmt in this change; option not taken: a first-heading rule for them, deferred to the fresh-bundle work.
- A title with surrounding spaces is aligned and compared trimmed, and its frontmatter line stays byte-identical.
- An empty or whitespace-only `title` counts as absent; option not taken: treating it as present, which doubled the number in the heading.

### Acceptance

- A committed hand-written ADR without timestamp gets the author date of its adding commit; an uncommitted one gets a timestamp within the test's window; a present timestamp is kept byte-for-byte.
- A missing owner on an ADR is seeded from git config; with no git identity it stays absent and `check` reports it; an issue without owner is left as it is.
- `# 0003. Other heading` under `title: Hand` becomes `# 0003. Hand`; every other body byte is identical, compared by hash; a record with no title in frontmatter gets it from its heading.
- `fmt --check` exits 1 listing the record and writes nothing; a second `fmt` writes nothing; `fmt` over this repository's `docs` changes nothing.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: timestamp, owner and heading repairs in `fmt`; tests red first, including the body-hash test.
2. CLI: `--check` reporting and help.
3. Topics and README.

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog.

- `cli/src/git.rs` (new): the git identity lookup moved from `new` and the adding-commit date (`git log --diff-filter=A --follow`, UTC); `cli/src/commands/fmt.rs` passes both to core as plain inputs, so core keeps no process or git I/O.
- `living-docs-core/src/commands/fmt/repair.rs` (new) and `fmt.rs`: fmt seeds a missing timestamp (adding commit, else the clock), a missing owner on a type that requires one, and a missing or empty title from the title heading, and aligns the title heading text with the trimmed frontmatter title; a present value is never changed and a retired record is left as history.
- `living-docs-core/src/commands/set/retitle.rs › title_heading_index`: one lookup for the title heading, shared by fmt and check's HEADING advisory: the first unfenced heading carrying the record's own number prefix, else the first unfenced unnumbered level-1 heading, else none. `set title` keeps its own behavior (tracked as issue 0074).
- Tests: `cli/tests/fmt/repair.rs` and `cli/tests/fmt/heading.rs` through the binary in temporary git repositories, core units in `fmt/heading_tests.rs` and `check/records/tests.rs`; the `check_sections_parser` fixture for a disagreeing title heading now carries its number. Planted mutations of each rule turned the tests red.
- Docs: README, `fmt --help`, the procedure, check and claude-hard-rules topics describe the repairs.
- Verification in the main checkout: `cargo test --workspace` 852 passed, 0 failed; `make check`, the file-size ratchet, clippy with `-D warnings`, `cargo fmt --check` and `se-gates check` pass; `fmt --check docs` lists nothing and `check docs` passes. Reviews: round 1 changes requested (a section heading was renamed), round 2 approved with two normal findings fixed by the owner's decision (level-1 fallback, trimmed title) and approved on narrow re-review.
- Found and opened separately: issues 0072 (CRLF frontmatter), 0073 (title derived for an untitled record), 0074 (`set title` heading).
