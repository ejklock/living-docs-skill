---
type: Issue
title: manage install mermaid has no end-to-end test of its success path
description: ADR 0085 asks for a stub curl serving the pinned bytes to prove the installed file lands under XDG_DATA_HOME; only a unit test with a made-up body and hash covers placement
status: open
timestamp: 2026-11-15T18:19:13Z
---

## 0129. manage install mermaid has no end-to-end test of its success path

### Scope

The Verification of ADR 0085 asks for a test where a stub `curl` serves the pinned bytes and `manage install mermaid` writes the file under `$XDG_DATA_HOME`. The review of issue 0127 slice S2 found that only the unit test of the download-and-verify function covers placement, with a made-up body and its own hash; the binary's success branch (the `mermaid 12.3.0 -> <path>` output) is never run by a test. Proving it end to end needs either the 5.5 MB pinned file as a fixture or a way for a test to make the CLI accept another hash, which widens the CLI surface.

### Decision

- Open: choose between a test-only seam for the expected hash, a cached copy of the pinned file used only in CI, or accepting the unit-level proof and amending the ADR's Verification wording.

### Acceptance

- Either the success branch of `manage install mermaid` is exercised by a test, or ADR 0085's Verification states the unit-level proof as sufficient, with the reason.
