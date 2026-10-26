---
type: Issue
title: a set read-back test writes a fixed-name temp file and fails when runs overlap
description: <One sentence — the change and its motivation.>
status: open
timestamp: 2026-10-25T02:00:37Z
---

## 0088. a set read-back test writes a fixed-name temp file and fails when runs overlap

Found by the review of the canonical supersedes work (2026-10-24): the test `crlf_adr_writes_equal_the_lf_twin_with_crlf` (cli/tests/set_read_back/crlf.rs) writes its section body to a fixed path in the system temp directory, `living-docs-set-read-back-section.txt`. Two test runs at once, or another test using the same name, race on that file, and the test fails with "cannot read" now and then.

### Scope

- Every test that hands `set ... --from` a file writes it under a path unique to that test run.

### Acceptance

- No test in cli/tests writes a fixed-name file in the shared temp directory.
- Two concurrent `cargo test -p living-docs --test set_read_back` runs both pass.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Tests: a unique scratch path helper for the set_read_back tests.
