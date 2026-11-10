---
name: living-docs-drift
description: Check whether the live records of a living-docs bundle still match the code they cite, and write a dated advisory drift report. Use when the owner asks whether the docs, architecture views, constitution, open issues or Accepted ADRs have drifted from the code. Runs a driver script that resolves each record's code anchors and asks a configurable judge command; never a gate.
version: "0.27.0"
metadata:
  type: skill
  layer: procedural
  tags: [documentation, drift, advisory, llm-judge]
---

# Living Docs Drift

Doc-to-code drift is a record that cites code that is gone, or describes behavior the code no longer has. `living-docs check` validates doc-to-doc invariants only; this skill covers the other half. The judgment comes from a model run outside the `living-docs` binary through a command you configure.

## When to use

The owner asks whether the docs still match the code, or wants a drift review before relying on a view, the constitution or an open issue. Never run it from a hook, pre-commit or CI job: the report is advisory only, never a gate.

## Run

From the repository root, with `bash` and `jq` 1.6 or newer on the PATH:

```
bash <skill dir>/scripts/drift.sh [--docs-dir DIR] [--root DIR] [--out DIR]
```

- `--docs-dir`: the bundle (default `<root>/docs`).
- `--root`: the repository root that code anchors resolve under (default `.`).
- `--out`: where the report goes (default `<root>/.living-docs/drift`, outside the bundle so `check` never sees it; the driver puts a `.gitignore` containing `*` in that default directory when it has none, so git never lists a report).

## Configuration

- `LIVING_DOCS_DRIFT_JUDGE`: the judge command line, run through the shell's own word splitting and quoting. Default `claude -p --tools ""`; `codex exec`, pi or any script that reads the prompt and payload on stdin and prints the answer on stdout also works. Set it only to commands you trust: it runs with your permissions.
- `LIVING_DOCS_DRIFT_TIMEOUT`: seconds allowed per judge call (default 300). A judge past it is killed with its child processes.

## What is judged

One judge call per in-scope record, found by reading the bundle's files directly:

- `anchors` mode: architecture views (`architecture/*.md`), the constitution, and issues whose `status:` is not `closed` or `done`. The report lists each cited anchor's result.
- `adherence` mode: ADRs with status `Accepted`. The judge asks only whether the code still follows the decision; the report never lists anchor results, because a path an ADR cites is history.
- Not sent: superseded, deprecated and proposed ADRs, closed and done issues.

An anchor is a backticked repository path, optionally followed by ` › ` and a backticked symbol, outside fenced code blocks (at most 20 per record). Results:

- `present`: the path exists under the root (and, with a symbol, the whole word is in the file).
- `name_absent`: the file exists but the symbol is not in it as a whole word. A word search cannot tell a definition from a mention, so `present` never claims the symbol is defined.
- `path_missing`: nothing at that path.
- `unresolvable`: an absolute path, a `..` path or a symlink leaving the root. It is never read.

Each readable cited file contributes its first 200 lines to the payload (`truncated: true` past that).

## Exit codes

- `0`: a report was written, whatever it found.
- `2`: no report. Bad usage, a missing or pre-1.6 `jq`, docs directory or judge command, a judge that exits non-zero or times out. The stderr line names the cause. An absent judgment is never read as a clean one.

## Read the report

`<out>/<YYYY-MM-DD>-drift-report.md`; a second run the same day replaces it. The header records the full judge command line, so never put a token or secret in that command; pass it through the environment instead. Each record section gives the record path (relative to the bundle), the SHA-256 of the exact payload sent, the anchor results (anchors mode only) and the verdict:

- `no_drift`: the judge found nothing.
- `drift`: findings, each of kind `missing_feature`, `incomplete_model`, `logic_shortcut` or `architectural_divergence`, with a summary and a cited path.
- `invalid answer`: the judge's reply failed the answer schema. It is not a clean result: rerun, or fix the judge.

A finding is a lead, not a verdict. Open the record and the cited code before acting on it; judgment is not reproducible between runs.
