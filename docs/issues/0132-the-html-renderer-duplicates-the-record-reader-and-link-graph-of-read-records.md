---
type: Issue
title: The HTML renderer duplicates the record reader and link graph of read records
description: Share one crate-internal reader and inbound-link graph between read records and the HTML renderer
status: open
timestamp: 2026-11-15T21:22:33Z
---

## 0132. The HTML renderer duplicates the record reader and link graph of read records

### Scope

The HTML renderer (`living-docs-core/src/html.rs`, issue 0127 slice S1) mirrors the record reader of `commands/effective.rs` (the read loop and `is_reserved`, about lines 136-153) and recomputes the inbound-link graph that `commands/effective/dependents.rs` keeps private (`key_of`, `inbound_links`), because the slice could not touch those files. `check::records::is_reserved` is already `pub(crate)`. Found by the review of that slice.

### Decision

- Expose one crate-internal reader and one inbound-link graph, used by `read records`, `--links-to`/`--impact` and the HTML renderer, so the three never disagree on which records exist and who links whom.

### Acceptance

- The renderer and `read records` share the reader and the graph; the duplicated functions are gone; `cargo test --workspace` and the jscpd duplication gate stay green.
