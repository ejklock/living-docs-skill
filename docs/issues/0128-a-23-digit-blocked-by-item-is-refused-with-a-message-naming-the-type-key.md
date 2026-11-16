---
type: Issue
title: A 23-digit blocked_by item is refused with a message naming the type key
description: The read-back guard refuses a very large number but names the wrong key in its message
status: closed
timestamp: 2026-11-15T18:06:57Z
---

## 0128. A 23-digit blocked_by item is refused with a message naming the type key

### Scope

`living-docs write set <issue> blocked_by 99999999999999999999999` (a 23-digit item) exits 2, which is safe, but the message says the value cannot be written "without changing how 'type' reads": it names the wrong key. The digit rule accepts the item, and the write path's read-back guard then refuses it, apparently because YAML reads a number that large differently. Found by the review of issue 0125 slice S2, using both the base 0.28.0 build and the slice build, so it predates that slice. The cause is not verified.

### Decision

- Find why the read-back guard names `type`, and make the refusal name the key being written and the reason (a number too large to read back unchanged).

### Acceptance

- The 23-digit `blocked_by` item is refused with exit 2 and a message naming `blocked_by` and the item; a test proves it.
- `cargo test --workspace` and `make check` stay green.

### Outcome

- A blocked_by item too large to read back as a number is now written quoted and round-trips as a string, so the misleading refusal naming `type` no longer occurs; the read-back guard message names the written keys when a draft no longer parses. Proof: `cli/tests/set_blocked_by_numbers.rs` › `blocked_by_round_trips_an_unreadable_number_quoted`, `supersede/tests.rs` guard-message test.
