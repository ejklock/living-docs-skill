---
type: Issue
title: Walking skeleton — Cargo workspace + living-docs-core + fs-store + thin cli
description: Extract the standalone cli crate into a workspace with a hexagonal core and an fs-store adapter, preserving all current behavior, and wire CI — the runnable foundation every later slice stacks on.
status: open
labels: [slice, skeleton, refactor]
blocked_by: []
tracker:
timestamp: 2026-07-16T00:00:00Z
---

## Walking skeleton — Cargo workspace + living-docs-core + fs-store + thin cli

Implements [ADR 0002](/adr/0002-hexagonal-core-workspace.md). Serves the
[Constitution](/constitution.md) North Star by laying the runnable foundation for
findability. **Slice 0** — the walking skeleton: does almost nothing new, but is end-to-end
and demoable, and every later slice keeps its smoke test green.

### Objective link

Constitution (findability) → this skeleton makes the core reusable by the DB + web slices →
ADR 0002 (hexagonal core in a workspace).

### Context manifest

- Read: `cli/src/main.rs`, `cli/src/{frontmatter,paths,templates}.rs`,
  `cli/src/commands/*.rs`, `cli/src/check/*.rs`, `cli/tests/*.rs`, `cli/Cargo.toml`.
- Seams touched: introduce workspace root `Cargo.toml`; new crates `living-docs-core/`
  (domain + `DocStore` + `SearchIndex` traits) and `fs-store/` (`DocStore` over `.md`);
  reduce `cli/` to arg-parsing + adapter wiring.
- Pattern: ports-and-adapters. The core depends on no adapter/front.

### Scope

Move the domain (`frontmatter`, `paths`, `templates`, `commands`, `check`) into
`living-docs-core` behind the `DocStore` port; implement `fs-store`; keep `cli` behavior
identical. Add a `NOTICE` recording the OKF Apache-2.0 attribution (logic ported from
`GoogleCloudPlatform/knowledge-catalog`), with the LLM synthesize path omitted. **KEPT
identical:** every existing subcommand's output and exit code.

### Vertical Demo

- **Given** the workspace builds, **When** I run `living-docs check docs`, **Then** it
  prints `OK — N docs, no invariant violations` (exit 0) exactly as before.
- **Given** a temp docs dir, **When** I run `living-docs new adr "x"` then
  `living-docs check`, **Then** the scaffolded ADR passes check — unchanged behavior through
  the new core.

### Acceptance

- The existing `cli/tests/*.rs` integration tests pass **unchanged** (behavior-parity
  fitness function). — `verify_by: test`
- `cargo test -p living-docs-core` exercises the domain against an in-memory `DocStore`,
  touching no filesystem. — `verify_by: test`
- **Fitness function (dependency direction):** `living-docs-core`'s manifest declares no
  dependency on `cli` or `fs-store`; a build/arch check fails if the core imports an adapter
  or front. — `verify_by: command`
- CI runs fmt + clippy + test on the workspace and a smoke invocation of the built binary. —
  `verify_by: command`

### Out of scope

No database, no search, no web (slices 0002+). No new user-facing behavior.

### Plan

Workspace root → extract `living-docs-core` (domain + traits) → `fs-store` adapter → rewire
`cli` → add `NOTICE` → wire CI. Dispatched through the pipeline as one slice.
