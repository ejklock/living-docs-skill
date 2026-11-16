---
type: Issue
title: "Execute the authoring-core cut: delete db-store, web, export, migrate, --json, seal remnants and the write-gate hook, retire their records and green the gates"
description: Implementation slices for the authoring-core cut, one commit per slice, ending with a green CI and the retired records deprecated or closed
status: closed
timestamp: 2026-09-18T08:26:00Z
---

## Execute the authoring-core cut

Implements [ADR 0059](../adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md). Each slice below is one commit so any of them can be reverted alone.

### Scope

Kept: `new`, `set`, `supersede`, `index`, `check`, `fmt`, `effective`, `skill`, `hooks` (session teaching plus pre-commit doc-gate), the `fs-store` adapter, the `DocStore` port, the `okf-knowledge-format` and `research-artifacts` skills, the Dockerfile.dev toolchain image.

Removed: `db-store`, `web`, `pii`, `export`, `leak-gate`, `migrate`, `new --json`, `index --visibility`, `--backend`, `--engine`, the seal dependencies, the PreToolUse write-gate and its fixtures, the `public-export` skill, the `migration` topic, `docker-compose.yml` and `env.example`.

### Acceptance

- `living-docs --help` lists the nine kept verbs and nothing else.
- `cargo test`, `cargo clippy --all-targets -- -D warnings` and `make check` pass; the file-size and allow-inventory baselines contain no entry for a deleted file.
- The constitution scope names the authoring core and carries the re-entry clause; the deprecated ADRs carry the CLI-written callout; `living-docs check docs` passes.

### Plan

1. Record ADR 0059 and this issue.
2. Remove `db-store`, `web`, `db`/`search`, `--backend`/`--engine`, the compose files and Makefile targets.
3. Remove `export`, `leak-gate`, `pii`, `visibility` and the `public-export` skill.
4. Remove `migrate`, `new --json` sections, the seal dependencies; make templates placeholder-only.
5. Remove the write-gate hook, its wiring, fixtures and parity test; trim `hooks install`.
6. Amend the constitution, CLAUDE.md, README, CONTRIBUTING and the skill rules; deprecate and close the retired records; regenerate indexes.
7. Refresh the ratchet baselines and CI; push; open the pull request.
