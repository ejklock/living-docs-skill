#!/usr/bin/env bash
#
# decision-prompt.sh — opt-in PostToolUse hook that reminds the agent, once per
# commit cycle, to record a material choice in the active issue's Decision
# section. Never a gate: it always exits 0, never writes in the tree and never
# blocks; it only adds context.
#
# Inert unless LIVING_DOCS_DECISION_PROMPT=1. LIVING_DOCS_BUNDLE=<dir> sets the
# docs bundle (default docs, relative to the repository root). It speaks only
# after an Edit, Write or MultiEdit of a file outside the bundle, when no path
# inside the bundle changed in the working tree, and it has not already spoken
# for the current HEAD (kept under the git directory; a repository without
# commits uses a fixed marker). Every fact comes from git, so no jq and no sha
# tool are needed.

set -u

[ "${LIVING_DOCS_DECISION_PROMPT:-0}" = "1" ] || exit 0

INPUT="$(cat 2>/dev/null || true)"
TOOL_RE='"tool_name"[[:space:]]*:[[:space:]]*"(Edit|Write|MultiEdit)"'
[[ "$INPUT" =~ $TOOL_RE ]] || exit 0
FILE_RE='"file_path"[[:space:]]*:[[:space:]]*"(([^"\\]|\\.)+)"'
[[ "$INPUT" =~ $FILE_RE ]] || exit 0
FILE="${BASH_REMATCH[1]}"
FILE="${FILE//\\\"/\"}"
FILE="${FILE//\\\\/\\}"

DIR="${CLAUDE_PROJECT_DIR:-$PWD}"
CWD_RE='"cwd"[[:space:]]*:[[:space:]]*"([^"]+)"'
[[ "$INPUT" =~ $CWD_RE ]] && DIR="${BASH_REMATCH[1]}"

ROOT="$(git -C "$DIR" rev-parse --show-toplevel 2>/dev/null)" || exit 0
GITDIR="$(git -C "$DIR" rev-parse --absolute-git-dir 2>/dev/null)" || exit 0
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"
BUNDLE="${BUNDLE%/}"

# Resolve a path to its physical form; the nearest existing ancestor stands in
# for the part of a new file that does not exist yet.
physical_path() {
  local p="$1" rest="" dir
  while [ ! -d "$p" ]; do
    rest="/${p##*/}$rest"
    case "$p" in */*) p="${p%/*}" ;; *) p="." ;; esac
    [ -n "$p" ] || p="/"
  done
  dir="$(cd "$p" 2>/dev/null && pwd -P)" || return 1
  printf '%s%s' "${dir%/}" "$rest"
}

case "$FILE" in /*) ;; *) FILE="$DIR/$FILE" ;; esac
RESOLVED="$(physical_path "$FILE")" || exit 0
ROOT_REAL="$(physical_path "$ROOT")" || exit 0
case "$RESOLVED" in "$ROOT_REAL"/*) REL="${RESOLVED#"$ROOT_REAL"/}" ;; *) exit 0 ;; esac
case "$REL" in "$BUNDLE"/*) exit 0 ;; esac

CHANGED=0
while IFS= read -r -d '' entry; do
  code="${entry:0:2}"
  path="${entry:3}"
  origin=""
  case "$code" in R* | C* | *R | *C) read -r -d '' origin ;; esac
  case "$path" in "$BUNDLE"/*) exit 0 ;; esac
  case "$origin" in "$BUNDLE"/*) exit 0 ;; esac
  CHANGED=1
done < <(git -C "$ROOT" status --porcelain -z -uall 2>/dev/null)

[ "$CHANGED" = "1" ] || exit 0

KEY="$(git -C "$ROOT" rev-parse -q --verify HEAD 2>/dev/null)" || KEY="no-commits"
STATE="$GITDIR/living-docs-decision-prompt"
[ "$(cat "$STATE" 2>/dev/null)" = "$KEY" ] && exit 0
printf '%s\n' "$KEY" >"$STATE" 2>/dev/null || exit 0

BRANCH="$(git -C "$ROOT" branch --show-current 2>/dev/null)"
NUM_RE='(^|[^0-9])([0-9]{4})([^0-9]|$)'
LAST_RE='-([0-9]{4})([^0-9]|$)'
ISSUE="issue/NNNN"
if [[ "${BRANCH##*/}" =~ $LAST_RE ]]; then
  ISSUE="issue/${BASH_REMATCH[1]}"
elif [[ "${BRANCH##*/}" =~ $NUM_RE ]]; then
  ISSUE="issue/${BASH_REMATCH[2]}"
elif [[ "$BRANCH" =~ $NUM_RE ]]; then
  ISSUE="issue/${BASH_REMATCH[2]}"
fi

REASON="living-docs: a code file changed and no record did. If you made a material choice, write it into the active issue's Decision section with \`living-docs write set $ISSUE section Decision --from -\`, which replaces the whole section, so pass its current bullets plus the new one; promote it to an ADR only when a future reader would pay to rediscover it. Do not create a new record per change; otherwise carry on."
printf '{"hookSpecificOutput":{"hookEventName":"PostToolUse","additionalContext":"%s"}}\n' "$(printf '%s' "$REASON" | sed 's/\\/\\\\/g; s/"/\\"/g')"
exit 0
