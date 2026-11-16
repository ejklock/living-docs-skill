---
type: Issue
title: The canonical writer leaves zero-padded numeric strings unquoted
description: A string such as 0005 is written plain, which serde_yaml reads back as a string but YAML 1.1 readers read as the octal number 5
status: closed
timestamp: 2026-11-15T18:19:14Z
---

## 0131. The canonical writer leaves zero-padded numeric strings unquoted

### Scope

The canonical frontmatter writer (`living-docs-core/src/record/canonical.rs` › `quote_flow`) writes a digit-only string with a leading zero, such as `0005`, without quotes. serde_yaml reads it back as the string `0005`, so living-docs round-trips it, but a YAML 1.1 reader reads it as the octal number 5. Raised by the coder of issue 0125 slice S3; it affects every flow list the writer emits, including tags and labels.

### Decision

- Keep zero-padded digits such as `0005` and `0089` unquoted and close this issue: every reader of the bundle is YAML 1.2 (`serde_yaml`), and quoting them would change the canonical bytes of every `supersedes`, `superseded_by` and `blocked_by` value, so `check` (`check/canonical.rs`) would fail every existing adopter bundle until `manage fmt` (owner, 2026-11-16). Not taken: quote and migrate through `manage fmt`; quote and let `check` accept both forms (relaxes the byte-exact rule of ADR 0019).

### Acceptance

- The choice is recorded here; if quoting is chosen, `fmt` keeps existing records stable and a test proves the round trip.

### Outcome

- Closed without code: the Decision keeps the current output. Issue 0135 quotes only the scalars `serde_yaml` itself would read as another type, which no existing record holds.
