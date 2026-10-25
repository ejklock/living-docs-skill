---
type: Issue
title: fmt rewrites the title heading of a record titled with digits only on every run
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:00:31Z
---

## 0085. fmt rewrites the title heading of a record titled with digits only on every run

Found by the review of the canonical supersedes work (2026-10-24): a record whose title is the text `0001` gets its title heading rewritten by a second `fmt` to `# 0001. 0001`, so `fmt` is not idempotent on it. The same happens on the base before that work.

### Scope

- `fmt` aligns the title heading of a record whose title is all digits once, and a second `fmt` changes nothing.

### Acceptance

- An ADR numbered 0003 titled `0001`: after one `fmt` the heading reads `# 0003. 0001`, a second `fmt` is byte-identical, and `check` is clean.
- A title that starts with digits and continues with words (`2024 roadmap`) keeps today's heading.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the title-heading match `fmt` and `check` share, tests red first.
