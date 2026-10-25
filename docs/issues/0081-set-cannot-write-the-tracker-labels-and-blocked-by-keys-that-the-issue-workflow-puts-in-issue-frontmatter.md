---
type: Issue
title: set cannot write the tracker, labels and blocked_by keys that the issue workflow puts in issue frontmatter
description: The issue workflow says tracker metadata lives in frontmatter, but the CLI owns frontmatter and offers no way to write it.
status: closed
timestamp: 2026-10-10T19:16:42Z
---

## 0081. set cannot write the tracker, labels and blocked_by keys that the issue workflow puts in issue frontmatter

Found adopting 0.21.0 in a downstream bundle (relent). `skills/living-docs/rules/issue-workflow.md` (lines 7 and 10) says `labels`, `blocked_by` and `tracker` live in the issue's frontmatter, and the project's own rules say to backfill the tracker number there after publishing. The `new issue` template carries none of the three, and `living-docs set` accepts only `status`, `description`, `owner` and `title`.

Reproduction: `living-docs set issue/0001 tracker 12` exits 2 with "'tracker' is not a settable field; expected one of status, description, owner, title". The same for `labels`. The only path left is a hand frontmatter edit, which the procedure itself calls a process error. In relent, 83 issues already carry `tracker`, 190 carry `labels` and 198 carry `blocked_by`.

### Scope

Add `tracker`, `labels` and `blocked_by` as settable keys, with the value shapes the existing records use (a number or URL for `tracker`, a list for `labels` and `blocked_by`), and make `fmt` keep them in canonical order. Kept: the three keys optional, so `new issue` need not emit them.

### Acceptance

- `living-docs set issue/NNNN tracker 12` writes the key and `check` stays OK.
- `set` writes `labels` and `blocked_by` as lists from a comma-separated value and rejects the keys on other doc types.
- `fmt --check` is clean after a `set` of each key.
- The error text for an unsupported key, `set --help` and issue-workflow.md name the new keys.
- `living-docs check docs` stays OK.

### Outcome

Delivered 2026-10-24 on branch task/ek-usage-audit-backlog.

- `set` writes `tracker` (one line of text), `labels` and `blocked_by` (comma-separated lists written as flow sequences) on Issue records only; on any other type they are refused naming the type. `set --help` lists them among the settable keys; `skills/living-docs/rules/issue-workflow.md` teaches them.
- Owner decision: every `set` and `supersede` write passes one read-back guard in the shared write path. The draft is refused, with every file byte-identical, unless every key it did not write reads as before, the body is unchanged (a missing final newline is the one change allowed, as before), each written key reads as its rendered line does, and each given value reads back exactly (`0x10` stays the text `0x10`). `supersede` drafts both records before writing either and refuses a record superseding itself.
- Owner decision: a value holding a whitespace-then-`#` comment opener is quoted by the canonical serializer, so `set title "a #b"`, `tracker "12 #x"` and `labels "a #b"` read back whole; before, `a #b` was stored as `a`.
- Owner decision, changed mid-work from "refuse CRLF": CRLF records are supported (issue 0072, [ADR 0072](/adr/0072-records-keep-their-own-line-endings-crlf-is-read-and-written-as-crlf-and-a-mixed-ending-record-fails-check.md)). A record with mixed endings is refused by `set`, `supersede` and `fmt` before any write and fails `check`.
- Tests: `cli/tests/set_tracker_keys.rs`, `cli/tests/set_read_back.rs` (with `crlf.rs` and `mixed.rs`), `cli/tests/check_crlf.rs`, `cli/tests/fmt/mixed.rs`, and unit tests for the guard, the quoting and the issue keys. Reviewers planted a mutant for each rule (guard rules, quoting, help key list, mixed refusal, check filters, CRLF restore); each failed a test.
- Review: one full review and several narrow re-reviews; every blocking and normal finding was fixed. An integration fix on this branch scoped one CRLF test assertion to the files it converts, because `new` now also writes an LF bundle-root index.
- Verification in the main checkout: `cargo test --workspace` 947 passed, 0 failed; `make check` (file-size ratchet and allow-inventory included; the `record.rs` baseline tightened to 400), clippy with `-D warnings`, `cargo fmt --check`, `living-docs index` (no-op), `living-docs check docs`, `fmt docs --check` and `se-gates check` pass.
