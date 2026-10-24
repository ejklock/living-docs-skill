---
type: Issue
title: Implement fmt's repair of timestamp, owner and title heading
description: "Implements ADR 0071: fmt seeds a missing timestamp from the adding commit, a missing required owner from git config, and aligns the title heading, keeping every other body byte identical"
status: open
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
