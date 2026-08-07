---
type: Issue
title: "Responsibility split: one verb per module, sibling test files, and a hard file-size ratchet enforced by a deterministic check"
description: Mechanical responsibility split (verb-per-module in cli, phase modules in db-store sync, sibling test files) plus a 400-line hard cap enforced by a ratcheting check-file-size.sh — maintainability becomes a gate, not an advisory.
status: closed
timestamp: 2026-08-05T20:49:48Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Responsibility split: one verb per module, sibling test files, and a hard file-size ratchet enforced by a deterministic check

The three largest source files concentrate most of the codebase's reading cost: `db-store/src/sync.rs` (1,157 lines), `cli/src/main.rs` (1,085 — every verb, clap wiring, and inline tests in one file), `db-store/src/migration.rs` (706). The `check/` modules (150-400 lines each) show the readable shape the rest should have. The maintainer wants maintainability and code comprehension protected as a HARD rule, not an advisory — the codebase is growing (graph slices, `import`, MCP front are all queued). A constraint without an instrument is a vibe: this issue ships the rule AND its deterministic check. No new crates: modules and folders first (ADR 0002/0033 seams stay as they are); a crate split waits for a real seam.

### Scope

- **Layout rules (become CLAUDE.md hard rule 5):**
  - `cli/src/main.rs` holds only clap wiring and dispatch; every verb lives in `cli/src/commands/<verb>.rs`.
  - `db-store/src/sync.rs` splits by phase (`sync/records.rs`, `sync/relations.rs`, `sync/tags.rs`, orchestrator in `sync/mod.rs`).
  - A `#[cfg(test)]` module over ~100 lines moves to a sibling file via `mod tests;` (keeps private access, cleans the production file).
  - Hard cap: 30 lines per FUNCTION — the primary clarity instrument (Clean Code's limit targets functions, not files).
  - Hard cap: 300 lines per `.rs` file (counted without the sibling `tests.rs`, which has its own 300 cap).
- **The instruments:** (a) clippy `too_many_lines` with `clippy.toml` threshold 30, promoted to deny in CI, for the function cap (grandfathered oversized functions get targeted `#[allow(clippy::too_many_lines)]` with a shrinking inventory); (b) `scripts/check-file-size.sh` (wired into CI next to `check-version.sh`): fails when any `.rs` file exceeds 300 lines, with an explicit grandfather list that may only SHRINK (ratchet — a grandfathered file that grows fails the check; a file that drops under the cap leaves the list permanently).
- **Behavior freeze:** this is a mechanical move refactor. The full test suite (922 tests) is the characterization harness: green before, green after, no test semantics changed. Public CLI behavior byte-identical.
- KEPT: crate boundaries (core / fs-store / db-store / cli / web); the deep-module philosophy — one module per responsibility, not many shallow files; `migration.rs` may stay grandfathered if its shape resists a clean phase split (schema-per-table split is allowed, not required, this pass).
- Out of scope: any behavior change, rename of public items, new crates, `living-docs-core/check/*` (already conformant).

### Acceptance

- `cli/src/main.rs` is under 150 lines and contains no verb logic; every verb compiles from `cli/src/commands/`.
- `db-store/src/sync.rs` is replaced by a `sync/` module tree; each file under 300 lines.
- `scripts/check-file-size.sh` exits non-zero when a non-grandfathered `.rs` file exceeds 300 lines or a grandfathered file grows; exits zero on the post-refactor tree; runs in CI.
- `cargo clippy` runs with `too_many_lines` at threshold 30 denied in CI; every `#[allow(clippy::too_many_lines)]` carries an inventory entry that only shrinks.
- `cargo test --workspace` passes with the same test count (adaptations limited to `use` paths and `mod` declarations); `cargo fmt --check` and clippy stay clean.
- CLAUDE.md carries the layout rules as hard rule 5, pointing at the check as its instrument.

### Plan

1. R1 — `cli` split: verbs to `cli/src/commands/`, test mods to sibling files, `main.rs` reduced to wiring.
2. R2 — `db-store` split: `sync/` phase modules; migration split only if clean.
3. R3 — `scripts/check-file-size.sh` + grandfather list + CI wiring + CLAUDE.md hard rule 5.
