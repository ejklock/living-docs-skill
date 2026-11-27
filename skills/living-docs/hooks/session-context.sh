#!/usr/bin/env bash
#
# session-context.sh — SessionStart hook for the living-docs authoring contract
# (deterministic point-of-use teaching).
#
# Emits one context line stating the authoring contract plus the
# read-before-deciding and decision-recording rules, with the resolved CLI
# binary, so every session receives it at t=0.
# Also points core.hooksPath at .githooks/ when the repo ships one, arming the
# pre-commit doc-gate. Always exits 0.
#
# With LIVING_DOCS_PLUGIN=1 (the Claude Code plugin, whose hook fires in every
# project the user opens) it acts only where the docs bundle root
# ($LIVING_DOCS_BUNDLE, default docs) has an index.md whose frontmatter holds an
# okf_version: line, so a MkDocs, Docusaurus or plain-notes docs/ gets no
# output; it leaves git configuration alone
# (a plugin must not edit a user's repository settings), and, when no binary is
# found, points at the npm and install.sh installs instead of a source build.

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

# True when $1 starts with a --- frontmatter block holding an okf_version: line
# before the closing ---. Reads only the first lines; tolerates CRLF and one
# leading UTF-8 byte order mark.
has_okf_marker() {
  [ -f "$1" ] || return 1
  head -n 50 "$1" 2>/dev/null | awk '
    { sub(/\r$/, "") }
    NR == 1 { sub(/^\357\273\277/, "") }
    NR == 1 { if ($0 != "---") exit; next }
    $0 == "---" { exit }
    /^okf_version:/ { found = 1; exit }
    END { exit !found }'
}

PLUGIN=0
[ "${LIVING_DOCS_PLUGIN:-0}" = "1" ] && PLUGIN=1

if [ "$PLUGIN" = 1 ]; then
  BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"
  has_okf_marker "$ROOT/${BUNDLE%/}/index.md" || exit 0
else
  arm_githooks
fi

BIN="$(resolve_bin)"
if [ -n "$BIN" ]; then
  BIN_NOTE="CLI: $BIN"
elif [ "$PLUGIN" = 1 ]; then
  BIN_NOTE="CLI not installed — run \`npm install -g @ejklock/living-docs\` or \`install.sh\` from https://github.com/ejklock/living-docs-skill before authoring docs"
else
  BIN_NOTE="CLI not built — run \`make build\` (or \`cargo build --release --manifest-path cli/Cargo.toml\`) before authoring docs"
fi

printf 'living-docs: %s. Docs authoring contract: write ONLY the body below the closing --- of a record. Numbering, frontmatter, supersede links, and index rows are CLI-owned — `living-docs write new <type> "<title>" [--body PATH|-]`, `living-docs write set <TYPE/NNNN> <key> <value>` or `living-docs write set <TYPE/NNNN> <key> --from PATH|-` (a record is named TYPE/NNNN, e.g. `write set adr/0001 status Accepted`, never a bare number; `write set <TYPE/NNNN> section "<heading>" --from PATH|-` replaces one body section; `write set <TYPE/NNNN> number next` renumbers a record with its links and index row), `living-docs write supersede <old> <new>`, `living-docs manage fmt`; prose arrives through `--body`/`--from` as a file or stdin, never argv; these leave the indexes current, and `living-docs manage index` repairs an index after a merge or a hand edit. A hand-written index fails `living-docs check` at commit. A choice made mid-work goes into the active issue'\''s Decision section the moment you make it (`living-docs write set issue/NNNN section Decision --from -` replaces the whole section, so pass its current bullets plus the new one); write an ADR only when a future reader would pay to rediscover the choice. Read before you decide: `living-docs read records --topic <term>` lists what already governs a topic; before editing or superseding a record, run `living-docs read records --links-to <TYPE/NNNN>` (what links it) and `living-docs read records --impact <TYPE/NNNN>` (what a change reaches); `living-docs read guide <topic>` loads the authoring rules for a topic.\n' "$BIN_NOTE"

exit 0
