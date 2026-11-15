---
type: ADR
title: manage html renders the bundle into one local, read-only, deterministic HTML file
description: Add a manage operation that renders the whole bundle to one self-contained HTML file for the owner's local reading, with browser-side Mermaid, no server, no index and no publication, amending the constitution's scope
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-15T16:03:26Z
---

# 0084. manage html renders the bundle into one local, read-only, deterministic HTML file

## Context

The owner wants to browse the bundle in a browser: records by type and status, the supersede lineage, which records link to which, and the architecture views with their Mermaid rendered. Today the only reading surfaces are the Markdown files, `read records` and the MCP front ([ADR 0078](/adr/0078-an-mcp-front-serves-the-bundle-over-stdio-as-write-read-manage-and-check-tools-from-the-living-docs-mcp-subcommand.md)), all text.

The project already built and removed such a surface. [ADR 0059](/adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md) deleted the axum web front, the database read-model behind it and public export: about 16k lines and two thirds of the locked dependencies, serving no consumer in the authoring loop. The constitution lets a front return only as a workspace front ([ADR 0033](/adr/0033-new-consumers-are-fronts-in-the-workspace-never-new-repos-until-a-deploy-cadence-or-ownership-trigger-fires.md)) when a consumer needs cross-project search or an independently deployed surface, and it forbids a daemon and a database. [Research 0005](/research/0005-engineering-documentation-in-ai-assisted-development-living-architecture-domain-language-and-rules-decision-capture-issue-waves-and-a-local-html-view.md) compared the options: a static export from the `.md` tree, a content mirror for an external generator (Quartz, Zola, mdBook, MkDocs), and a local `serve` command.

The owner chose ([issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md)) a local single-file export that the owner reads, never published. That reader is outside the authoring loop, and a local file is neither cross-project search nor a deployed surface, so the constitution's clause does not cover it as written; this ADR amends the scope rather than stretching the clause.

## Decision

We will add `living-docs manage html`, which renders the bundle into one self-contained, read-only HTML file for local reading.

- **Surface.** A `manage` operation, not a top-level verb, keeping the surface of [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md). It is not exposed as an MCP tool: an agent reads Markdown.
- **Output.** One file, default `.living-docs/html/index.html` (the tool's existing ignored output directory), `--out PATH` to override. The tool never serves, uploads or publishes it.
- **Content.** Every record grouped by type and status from frontmatter; each record's body rendered from Markdown; backlinks and the supersede chain from the in-memory link graph of [ADR 0074](/adr/0074-read-links-to-and-impact-answer-who-depends-on-a-record-from-a-link-graph-built-in-memory-per-run.md); architecture views ([ADR 0036](/adr/0036-architecture-views-are-a-registry-doc-type-on-a-named-identity-with-a-kind-sequenced-generated-index.md)) in their kind order. A small inline script filters records by text; no search index is built.
- **Mermaid.** Fenced `mermaid` blocks are emitted as diagram source and rendered in the browser by a pinned release of mermaid.js (MIT) that the binary embeds and inlines into the file, so the page renders offline and needs no network. The pinned version changes only by a deliberate upgrade, recorded in the issue that carries it.
- **Determinism.** The same bundle produces byte-identical output: stable ordering, no build timestamp, no absolute path.
- **Dependencies.** Only the `html` feature of the existing `pulldown-cmark` dependency and the embedded mermaid.js asset; no template engine, no HTTP stack, no async runtime.
- **Scope amendment.** The constitution adds "a local, read-only, single-file HTML render of the bundle" to the scope and keeps "never served, never published" in the out-of-scope list.

Rejected alternatives:
- **Wait for a named reader outside the owner** (the advisor's recommendation). It keeps ADR 0059's bar untouched, but the owner is the reader and has asked for it.
- **A content mirror and configuration for Quartz or Zola.** Backlinks and a graph view come built in, but the tool would chase a third-party format that keeps changing (Quartz v5 rebuilt its configuration in 2026) and the owner would need Node or a second binary.
- **A `serve` command with live reload.** It brings back the HTTP stack and a long-running process ADR 0059 removed; re-running the export takes milliseconds.
- **mermaid.js from a pinned CDN URL with a subresource-integrity hash.** It keeps the binary small, but the page needs network access to render a diagram and loads code from a third party each time it opens; the owner chose offline rendering over binary size.
- **No rendering, diagrams shown as source.** No asset at all, but the architecture views, a reason to read the bundle in a browser, would stay unreadable as diagrams.
- **Pagefind for search.** Its Rust API runs on `tokio`, which the cut removed, and its index needs HTTP rather than a `file://` page.
- **Publishing the file.** It reopens the leak question of [ADR 0010](/adr/0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md), which is Deprecated; publishing needs its own record.

## Consequences

**Easier / gained:**
- The owner reads the whole bundle, its lineage and its diagrams in a browser, with no server and no build toolchain.
- Determinism makes the renderer testable: rendering twice gives identical bytes.

**Harder / accepted trade-offs:**
- A second rendering path to maintain, the periphery cost ADR 0059 cut; bounded by one module, one feature flag and no stored state.
- The binary grows by the size of the embedded mermaid.js on every release, and every output file carries it inline; the implementation issue measures both and sets a budget.
- The pinned mermaid.js needs a deliberate upgrade to pick up fixes, and it can disagree with the merman-core validator that check uses (issue 0051).
- The scope now holds one surface that no agent uses, justified by a human reader only.

**Follow-ups:**
- An issue to implement the operation, its tests and the constitution amendment, with a size budget measured on the bench bundle.

## Verification

**Implementation impact:** `living-docs-core/Cargo.toml` (the `html` feature), a new rendering module in `living-docs-core`, `cli/src/commands/` (the `manage html` wiring), `cli/tests/`, `docs/constitution.md`.

**Verification criteria:**
- Rendering the same bundle twice produces byte-identical files; the file contains no timestamp and no absolute path.
- A fixture bundle's output contains every record title, each superseded record links its successor, and a record linked by two others lists both as backlinks.
- A record body holding `<script>` text renders it escaped, never executed.
- `Cargo.lock` grows by at most the packages the `html` feature pulls in, and `living-docs --help` lists the same top-level verbs as before.
- The output file references no external URL: mermaid.js is inlined, and a fixture view's diagram renders with the network disabled.
