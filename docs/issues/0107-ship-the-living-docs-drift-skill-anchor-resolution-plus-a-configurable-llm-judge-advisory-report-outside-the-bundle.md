---
type: Issue
title: "Ship the living-docs-drift skill: anchor resolution plus a configurable LLM judge, advisory report outside the bundle"
description: Add a living-docs-drift skill whose driver resolves record anchors, asks a configurable judge command about drift and writes a dated advisory report, as ADR 0075 decides.
status: open
timestamp: 2026-11-09T03:17:16Z
---

## 0107. Ship the living-docs-drift skill: anchor resolution plus a configurable LLM judge, advisory report outside the bundle

## Ship the living-docs-drift skill: anchor resolution plus a configurable LLM judge, advisory report outside the bundle

Implements [ADR 0075](/adr/0075-doc-to-code-drift-is-an-advisory-skill-whose-llm-judge-runs-outside-the-binary-through-a-configurable-command.md): a new `skills/living-docs-drift/` skill whose driver script resolves each in-scope record's code anchors, asks a configurable judge command whether the code still matches the record, validates every answer and writes a dated advisory report outside the bundle. The binary changes only so `install skills` places the new skill.

### Scope

- New `skills/living-docs-drift/`: `SKILL.md` (trigger, how to run, how to read the report, the configuration), `scripts/drift.sh` (the driver), `prompt.md` (the judge instructions), `tests/run.sh` with fixtures and stub judges.
- `cli/src/skill_install.rs` gains the asset entry for the new skill; `cli/tests/skill_install.rs` covers it.
- `Makefile` wires the new suite into `make check`, as `test-fixtures` does for `skills/living-docs/tests/run.sh`.
- Kept: no verb, flag, JSON field or check pass is added to the binary; no hook, pre-commit or CI job runs the driver.

### Decision

- **Language: bash plus `jq`**, like the repository's other scripts and fixtures, rather than a Rust front (ADR 0075 rejects it).
- **Configuration by environment**: `LIVING_DOCS_DRIFT_JUDGE` holds the judge command line (default `claude -p --tools ""`), `LIVING_DOCS_DRIFT_TIMEOUT` the per-call seconds (default 300); `--docs-dir`, `--root` and `--out` flags match the CLI's names. Not chosen: a TOML file like the curator's, which a single-command script does not need.
- **Status from frontmatter**: the script reads `status:` from each record file and keeps the terminal issue statuses (`closed`, `done`) in its own list, because `read --json` carries no status and adding one is a binary change ADR 0075 excludes.
- **Excerpt bound**: each cited file contributes at most its first 200 lines, at most 20 anchors per record; a truncated excerpt says so in the payload.
- **One fenced block tolerated**: a judge answer wrapped in a single ```` ```json ```` fence is unwrapped before validation; anything else that is not the bare JSON object is an invalid answer.

### Acceptance

- In scope: architecture views, the constitution and issues with a non-terminal status are judged in `anchors` mode; Accepted ADRs in `adherence` mode; superseded, deprecated, closed, done and other records are not sent. A fixture bundle with one record of each proves it by the payloads the stub judge receives.
- Anchor resolution, deciding cases: an existing path with a symbol present as a whole word → `present`; the same path with the symbol only as a substring of a longer word → `name_absent`; a missing path → `path_missing`; `a::b::sym()` normalizes to `sym`; an absolute path, a `..` path and a symlink resolving outside the root → `unresolvable`, and that file's content is absent from the payload.
- Adherence mode never reports an anchor finding in the report, even for a missing path.
- The payload is a JSON object with `schema_version` 1, `mode`, `record` (type, number or null, title, path, status), `body`, `anchors` and `excerpts`; a record body containing the text `ignore previous instructions` and a backtick-heavy code excerpt reach the judge as JSON string values, after the prompt's data-not-instructions marker.
- Judge answer, deciding cases: a valid `no_drift` with empty findings; a valid `drift` with one finding of each of the four kinds; the same answer inside one `json` fence → accepted; `drift` with empty findings, `no_drift` with findings, an unknown kind, a wrong `schema_version`, prose instead of JSON and an empty answer → each reported as an invalid answer for that record, never as no drift, and the run still exits 0.
- Fail direction: a judge command not found, a judge exiting non-zero, a judge exceeding the timeout and a missing `jq` each exit 2 with a stderr line naming the cause and leave no report file.
- Report: written atomically to `<out>/<YYYY-MM-DD>-drift-report.md`, default `out` `.living-docs/drift` under the root; its header names the judge command; each record section names the record path, the payload SHA-256, the anchor results (anchors mode only) and the judge verdict with its findings. A second run the same day replaces the file. `living-docs check` over the fixture bundle stays green after a run.
- `living-docs install skills --dir <tmp>` places `living-docs-drift/SKILL.md`, `scripts/drift.sh` and `prompt.md`; `make version` passes with the new `SKILL.md`; `living-docs --help` still lists ten verbs.
- `make check` runs the new suite and passes; `se-gates check` passes.
