---
type: Issue
title: A blocked_by item beyond the u64 range is refused with a message naming the wrong key
description: The early item check misses digits-only items serde_yaml cannot read as numbers, so the generic read-back message names type
status: open
timestamp: 2026-11-15T22:51:24Z
---

## 0136. A blocked_by item beyond the u64 range is refused with a message naming the wrong key

### Scope

`living-docs write set <issue> blocked_by "12, 99999999999999999999"` (a digits-only item beyond the u64 range) exits 2 with the write path's generic read-back message, `... cannot be written without changing how 'type' reads in the record's frontmatter`, which names neither the item nor the key being written. The early item check added by issue 0133 does not catch it because serde_yaml does not read it as a number. Found by the review of that issue; it predates it and is related to issue 0128.

### Decision

- Refuse or accept such an item through the same early check, naming the item and the key; decide together with issue 0128.

### Acceptance

- The command either writes the item or refuses it naming `blocked_by` and the item; a test proves it.
