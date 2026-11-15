---
type: Issue
title: "Implement ADR 0084 and 0085: manage html renderer, manage install mermaid and the CLI wiring"
description: "Land ADR 0084 as amended by ADR 0085: the deterministic core renderer, the hash-verified mermaid install target and the manage html operation"
status: open
timestamp: 2026-11-15T17:33:57Z
---

## 0127. Implement ADR 0084 and 0085: manage html renderer, manage install mermaid and the CLI wiring

### Scope

Implements [ADR 0084](/adr/0084-manage-html-renders-the-bundle-into-one-local-read-only-deterministic-html-file.md) and [ADR 0085](/adr/0085-manage-install-mermaid-fetches-the-pinned-mermaid-js-with-curl-and-manage-html-inlines-it-when-installed.md): `manage html` renders the bundle into one local, read-only, deterministic HTML file, and `manage install mermaid` installs the pinned, hash-verified mermaid.js it inlines. Slice S7 of [issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

Plan:
- S1: `living-docs-core::html::render` returns one deterministic HTML document: records grouped by type and status, bodies rendered from Markdown with raw HTML escaped, backlinks and the supersede chain from the ADR 0074 link graph, architecture views in kind order, an inline text filter, mermaid fences as diagram source, and mermaid.js from the given `MermaidSource` | files: living-docs-core/Cargo.toml (`html` feature), Cargo.lock, living-docs-core/src/html.rs (scaffolded), living-docs-core/src/html/ (submodules), living-docs-core/src/lib.rs | after: none
- S2: `manage install mermaid` fetches the pinned file with `curl` into `$XDG_DATA_HOME/living-docs/mermaid/12.3.0/`, verifies SHA-256 `336df803d87a38b70a01e736e666c8f7601d2ce42f5e47c8fafe5d721074b9a3`, and honours `--dry-run` | files: cli/src/args/sub.rs, cli/src/commands/install.rs, a new cli/src/commands module, cli/tests/install_mermaid.rs (create) | after: none
- S3: `manage html [--out PATH] [--mermaid PATH]` writes the file (default `.living-docs/html/index.html`), choosing mermaid.js from `--mermaid`, then the installed copy re-verified by hash, then the pinned CDN URL with integrity `sha384-HHW54r639fK42Ip0coGF3mz8oSiGn0eEoWNTLs75X7H63pLSDBa0Tqs0RwGmLlXE`, and naming the source on stderr | files: cli/src/args/sub.rs, a new cli/src/commands/html.rs, cli/src/commands/mod.rs, cli/tests/manage_html.rs (create) | after: S1, S2
- S4 (session): amend the constitution's scope with the local single-file render, never served or published | files: docs/constitution.md | after: S3

### Decision

- S2 and S3 both edit `cli/src/args/sub.rs`, so S3 waits for S2.
- The renderer escapes through `pulldown_cmark_escape` as a direct dependency rather than a local helper; it adds no package to Cargo.lock (review of slice S1). Not taken: the hand-written escape.
- The core renderer takes the mermaid source as data and never reads the network or the data directory; the CLI resolves the source.

### Acceptance

- Each Verification criterion of ADR 0084 (as amended by ADR 0085) and of ADR 0085 holds.
- `Cargo.lock` gains only the packages the `html` feature pulls in.
- `cargo test --workspace`, `make check`, `cargo fmt --all --check` and `se-gates check` pass.
