---
type: Issue
title: blocked_by cannot hold an item YAML reads as another type
description: write set refuses items such as +4, 1.5, true, null, 0x10 or 1e3 because the canonical writer would emit them unquoted and they would read back as different values
status: closed
timestamp: 2026-11-15T22:43:00Z
---

## 0135. blocked_by cannot hold an item YAML reads as another type

### Scope

`write set <issue> blocked_by` refuses items that YAML reads as a different type when written plain: signed or decimal numbers, booleans, null, hex and exponent forms. Writing `+4` plain would read back as the issue number 4. The refusal is safe, and after issue 0133 slice S1 it names the item, but such an item can then only be written by hand. Found by the review of that slice. The canonical writer (`living-docs-core/src/record/canonical.rs`, `needs_quoting`) decides when a flow item is quoted.

### Decision

- Quote such items in the canonical writer so they round-trip as strings: `write set` accepts `+4`, writes `"+4"`, and `check` advises on it as a non-numeric item (owner, 2026-11-16). Not taken: keep refusing them because no observed bundle uses them; defer.
- Delivered together with issue 0131, which needs the same quoting rule.

### Acceptance

- The choice is recorded here; if quoting is chosen, `write set` accepts `+4` and writes `"+4"`, it reads back as the string `+4`, and `check` advises on it as a non-numeric item.

### Outcome

- `write set` writes `blocked_by` and `labels` through a strict flow list that quotes every item YAML would read as another type (`+4`, `true`, `1.5`, `1e3`, `0x10`, `null`), so such items are accepted and `check` advises on them. Values the previous writer accepted keep their bytes (180 key/value probes against the previous binary, none changed), and zero-padded digits stay plain per issue 0131. Proof: `record/tests/quoting.rs`, `cli/tests/set_blocked_by_numbers.rs`, `cli/tests/set_tracker_keys.rs`.
- Tracked separately: a purely numeric label such as `12` is still refused (issue opened 2026-11-16).
