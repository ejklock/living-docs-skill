---
type: Issue
title: manage install mermaid has no end-to-end test of its success path
description: ADR 0091 asks for a stub curl serving the pinned bytes to prove the installed file lands under XDG_DATA_HOME; only a unit test with a made-up body and hash covers placement
status: closed
timestamp: 2026-11-15T18:19:13Z
---

## 0129. manage install mermaid has no end-to-end test of its success path

### Scope

The Verification of ADR 0091 asks for a test where a stub `curl` serves the pinned bytes and `manage install mermaid` writes the file under `$XDG_DATA_HOME`. The review of issue 0127 slice S2 found that only the unit test of the download-and-verify function covers placement, with a made-up body and its own hash; the binary's success branch (the `mermaid 12.3.0 -> <path>` output) is never run by a test. Proving it end to end needs either the 5.5 MB pinned file as a fixture or a way for a test to make the CLI accept another hash, which widens the CLI surface.

### Decision

- Accept the unit-level proof: `cli/src/mermaid_asset/tests.rs` › `matching_bytes_are_placed_at_the_destination` runs the download-and-verify path through a stub `curl` and proves placement, and `cli/tests/install_mermaid.rs` proves every failure branch and `--dry-run` through the binary. Only the binary's success line is unexercised, because the expected hash is the pinned release's and the pinned bytes are 5.5 MB. ADR 0091's Verification now says so (owner, 2026-11-16). Not taken: a test-only seam for the expected hash (an injection point that exists only for a test); defer.

### Acceptance

- Either the success branch of `manage install mermaid` is exercised by a test, or ADR 0091's Verification states the unit-level proof as sufficient, with the reason.

### Outcome

- Closed without code: the Decision accepts the unit-level proof and ADR 0091's Verification now states it, with the reason.
