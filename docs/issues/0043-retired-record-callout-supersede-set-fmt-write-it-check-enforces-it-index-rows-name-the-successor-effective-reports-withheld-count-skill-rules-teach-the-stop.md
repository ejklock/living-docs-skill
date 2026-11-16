---
type: Issue
title: "Retired-record callout: supersede/set/fmt write it, check enforces it, index rows name the successor, effective reports withheld count, skill rules teach the stop"
description: Implement the retired-record self-declaration decided in ADR 0058 across core, check, index, effective, the skill corpus, and this repo's own docs bundle.
owner: Evaldo Klock
status: closed
timestamp: 2026-09-17T14:49:17Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Retired-record callout across the CLI, the gate, the read surfaces, and the skill corpus

An agent that opens a Superseded or Deprecated record directly sees no visible sign that the record is history, and plans on a dead decision. Implements [ADR 0058](../adr/0058-retired-records-declare-themselves-a-cli-written-callout-in-the-body-successor-bearing-index-rows-and-effective-as-the-agent-read-verb.md).

### Scope

- Core: one `callout` module (expected text per status, detection, body reconcile). `supersede`, `set status`, and `fmt` call it. `fmt --check` reports a pending callout.
- Gate: a `check` pass that fails a retired record without its exact callout and an active record with one.
- Read surfaces: `index.md` retired rows name the successor (`Superseded by [NNNN](file)` / `Deprecated (no successor)`) under a fixed history note; `effective` opens with the withheld count when retired records exist.
- Skill corpus: `templates/claude-hard-rules.md` §10, `SKILL.md`, `rules/adr-conventions.md`, `rules/semantic-index.md`, `rules/procedure.md`, `rules/check.md`, `templates/adr.md`, `templates/prd.md` teach the stop rule and name the callout as CLI-owned.
- Dogfood: this repo's 17 retired ADRs and `examples/linkly` get the callout through the new `fmt`; every `index.md` regenerates; `check docs` stays green.
- Kept: `Superseded` / `Deprecated` vocabulary, `## Active` / `## Superseded` headings, record numbers and paths, ADR 0050's `Deprecated` status.
- Out: a set-valued `supersede`, a web visual treatment for the callout, status-vocabulary renames.

### Acceptance

- `supersede 0001 0002` makes 0001's body open with the Superseded callout linking 0002's filename; 0002 carries no callout.
- `set <ref> status Deprecated` adds the Deprecated callout; a later `set` to an active status removes it.
- `fmt` adds a missing callout, removes a stale one, reports both under `--check`, and is byte-identical on a second run.
- `check` reports a violation for a retired record without its callout and for an active record with one; a conformant bundle stays OK.
- `index` renders `- Superseded by [NNNN](file)` and `- Deprecated (no successor)` rows under the fixed history note; active rows are unchanged; the output is idempotent.
- `effective` opens with `_Withheld N retired record(s) …_` when N ≥ 1 and prints nothing extra when N = 0.
- db-mode `supersede_checked` yields the same old-record content as the fs path.
- After `fmt docs`, `fmt examples/linkly/docs`, and `index`, `living-docs check docs` and the fixture check exit 0.
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `./scripts/check-file-size.sh`, and `cargo test` pass; `living-docs-core/src/commands/index.rs` and `record.rs` do not grow.

### Plan

1. `living-docs-core/src/callout.rs` + `callout/tests.rs`: pure model + store-level reconcile.
2. `supersede` and `set` call reconcile after the frontmatter write; db-store fixture updated.
3. `fmt` reconciles the callout inside its pending computation.
4. `check/callout.rs` pass, registered in `check/mod.rs`.
5. `index`: row rendering extracted to `index/rows.rs`; `Record` learns `superseded_by`; retired rows and the note.
6. `effective`: withheld line.
7. Skill corpus edits.
8. Dogfood sweep with the freshly built binary; regenerate indexes; `check docs` green.

### Outcome

Closed: the retired-record callout (supersede/set/fmt writing it, check enforcing it, index and effective reporting it) shipped in v0.17.0.
