---
type: ADR
title: manage install mermaid fetches the pinned mermaid.js with curl and manage html inlines it when installed
description: Replace embedding mermaid.js in the binary with an on-demand, hash-verified install through curl, inlined by manage html when present and falling back to the pinned CDN URL with an integrity hash
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-15T17:30:13Z
---

# 0091. manage install mermaid fetches the pinned mermaid.js with curl and manage html inlines it when installed

## Context

[ADR 0090](/adr/0090-manage-html-renders-the-bundle-into-one-local-read-only-deterministic-html-file.md) decided that `manage html` embeds mermaid.js in the binary and inlines it into every output file, so diagrams render offline. The size was not measured when that was decided. Measured on 2026-11-15: `mermaid.min.js` 12.3.0 (MIT) is 5,537,838 bytes (SHA-256 `336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3`, identical in the npm tarball and on jsDelivr), against a 15.4 MB release binary: embedding grows the binary by 36% on every release, and the file would have to be committed to the repository.

The owner still wants offline rendering, without the weight ([issue 0144](/issues/0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md)). Three ways to fetch the file on demand were compared: the install channel (`install.sh` and the npm package) downloading it beside the binary, a `manage install mermaid` operation that shells out to `curl`, and the same operation with an HTTP client compiled into the binary. [ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) removed the HTTP stack from the workspace, and [ADR 0075](/adr/0075-doc-to-code-drift-is-an-advisory-skill-whose-llm-judge-runs-outside-the-binary-through-a-configurable-command.md) set the precedent of shelling out to a configurable external command instead of compiling one in.

## Decision

We will install mermaid.js on demand with `living-docs manage install mermaid`, which shells out to `curl`, and `manage html` will inline the installed copy when it is present.

- **Install.** `manage install mermaid` downloads the pinned `https://cdn.jsdelivr.net/npm/mermaid@12.3.0/dist/mermaid.min.js` with `curl -fsSL` into `$XDG_DATA_HOME/living-docs/mermaid/12.3.0/mermaid.min.js` (default `~/.local/share`), verifies its SHA-256 against the value pinned in the binary, and removes the download and exits non-zero on a mismatch, a missing `curl` or a failed transfer. It honours `--dry-run` like the other `manage install` targets. The pinned version and hash change only by a deliberate upgrade.
- **Render.** `manage html` takes mermaid.js from, in order: `--mermaid PATH` (inlined as given); the installed pinned copy, re-verified by hash before inlining; otherwise a `<script>` tag for the same pinned jsDelivr URL with the subresource-integrity hash `sha384-HHW54r639fK42Ip0coGF3mz8oSiGn0eEoWNTLs75X7H63pLSDBa0Tqs0RwGmLlXE`, which needs the network. The run says on stderr which source it used.
- **Determinism.** For one bundle and one mermaid source the output stays byte-identical.
- **Dependencies.** No new package: the hash uses `sha2`, already in the build through `rmcp` and `rust-embed`.

This supersedes only the Mermaid bullet of ADR 0090; the rest of ADR 0090 stands.

Rejected alternatives:
- **Embed mermaid.js in the binary** (ADR 0090 as accepted). Offline with no install step, but +5.5 MB on every release and a 5.5 MB file in the repository.
- **Embed it compressed** (1.6 MB with gzip). A third of the weight, but still a binary blob in the repository and a decompression dependency.
- **Download through the install channel.** No new operation, but a binary built with `cargo build` or copied by hand would never get the file.
- **An HTTP client inside the binary.** `ureq` with `rustls` adds dozens of packages, the weight ADR 0059 removed.
- **CDN only.** Zero weight, but no offline rendering, which the owner asked for.

## Consequences

**Easier / gained:**
- The binary and the repository do not grow; the HTML file stays small unless mermaid.js is inlined.
- Rendering is offline after one install, and a pinned hash guards the file against a changed or compromised CDN.

**Harder / accepted trade-offs:**
- One more step for offline diagrams, and `curl` must be on the PATH for it.
- A new `manage install` target widens the CLI surface of [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md); it follows the existing `manage install` shape, so no verb is added.
- An inlined file is about 5.5 MB per output.

**Follow-ups:**
- The implementation issue of ADR 0090 carries the install target as its own slice.

## Verification

**Implementation impact:** `cli/src/args/sub.rs` (the `mermaid` install target and the `--mermaid` flag), a new `cli/src/commands/` module for the install target, the `manage html` renderer's mermaid source lookup, `cli/tests/`.

**Verification criteria:**
- With a stub `curl` on PATH serving the pinned bytes, `manage install mermaid` writes the file under `$XDG_DATA_HOME`; serving altered bytes exits non-zero and leaves no file; no `curl` on PATH exits non-zero naming it; `--dry-run` writes nothing.
- `manage html` inlines `--mermaid PATH` when given, the installed copy when present and intact, and otherwise emits the pinned URL with its integrity attribute; a tampered installed copy is refused, not inlined.
- `Cargo.lock` gains no package.
