#!/usr/bin/env bash
#
# session-context.sh — SessionStart hook for the living-docs authoring contract
# (deterministic point-of-use teaching).
#
# Emits one context line stating the body-only rule and the resolved CLI binary,
# so every session receives the rule at t=0 instead of behind a skill trigger.
# Also points core.hooksPath at .githooks/ when the repo ships one, arming the
# pre-commit doc-gate. Always exits 0.

set -u

ROOT="${CLAUDE_PROJECT_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd)}"

resolve_bin() {
  if command -v living-docs >/dev/null 2>&1; then
    command -v living-docs
  elif [ -x "$ROOT/target/release/living-docs" ]; then
    printf '%s' "$ROOT/target/release/living-docs"
  fi
}

arm_githooks() {
  [ -d "$ROOT/.githooks" ] || return 0
  git -C "$ROOT" config core.hooksPath .githooks 2>/dev/null || true
}

arm_githooks

BIN="$(resolve_bin)"
if [ -n "$BIN" ]; then
  BIN_NOTE="CLI: $BIN"
else
  BIN_NOTE="CLI not built — run \`make build\` (or \`cargo build --release --manifest-path cli/Cargo.toml\`) before authoring docs"
fi

printf 'living-docs: %s. Docs authoring contract: write ONLY the body below the closing --- of a record. Numbering, frontmatter, supersede links, and index rows are CLI-owned — `living-docs new <type> "<title>" [--body PATH|-]`, `living-docs set <TYPE/NNNN> <key> <value>` or `living-docs set <TYPE/NNNN> <key> --from PATH|-` (a record is named TYPE/NNNN, e.g. `set adr/0001 status Accepted`, never a bare number; `set <TYPE/NNNN> section "<heading>" --from PATH|-` replaces one body section; `set <TYPE/NNNN> number next` renumbers a record with its links and index row), `living-docs supersede <old> <new>`, `living-docs fmt`; prose arrives through `--body`/`--from` as a file or stdin, never argv; these leave the indexes current, and `living-docs index` repairs an index after a merge or a hand edit. A hand-written one fails `living-docs check` at commit.\n' "$BIN_NOTE"

exit 0
