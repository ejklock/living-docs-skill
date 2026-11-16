---
type: Issue
title: The canonical writer leaves zero-padded numeric strings unquoted
description: A string such as 0005 is written plain, which serde_yaml reads back as a string but YAML 1.1 readers read as the octal number 5
status: open
timestamp: 2026-11-15T18:19:14Z
---

## 0131. The canonical writer leaves zero-padded numeric strings unquoted

### Scope

The canonical frontmatter writer (`living-docs-core/src/record/canonical.rs` › `quote_flow`) writes a digit-only string with a leading zero, such as `0005`, without quotes. serde_yaml reads it back as the string `0005`, so living-docs round-trips it, but a YAML 1.1 reader reads it as the octal number 5. Raised by the coder of issue 0125 slice S3; it affects every flow list the writer emits, including tags and labels.

### Decision

- Quote, in the canonical writer, every plain scalar a YAML 1.1 or 1.2 reader would read as another type, a digit-only string with a leading zero such as `0005` included, so the bundle round-trips through any reader (owner, 2026-11-16). Not taken: keep the plain output because only YAML 1.2 readers consume the bundle.
- Delivered together with issue 0135, which needs the same quoting rule.

### Acceptance

- The choice is recorded here; if quoting is chosen, `fmt` keeps existing records stable and a test proves the round trip.
