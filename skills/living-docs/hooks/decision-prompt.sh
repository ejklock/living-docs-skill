#!/usr/bin/env bash
#
# decision-prompt.sh — opt-in Stop hook that asks the agent, once per distinct
# set of code changes, to record a material choice in the active issue's
# Decision section. Never a gate: it always exits 0 and never writes in the tree.
#
# Inert unless LIVING_DOCS_DECISION_PROMPT=1. LIVING_DOCS_BUNDLE=<dir> sets the
# docs bundle (default docs). It blocks only when paths outside the bundle
# changed, no path inside the bundle changed, no Stop hook already continued
# this turn, and the changed paths with their content hashes differ from the
# last set it prompted for (kept under the git directory). Every fact comes
# from git, so no jq and no sha tool are needed.

set -u

[ "${LIVING_DOCS_DECISION_PROMPT:-0}" = "1" ] || exit 0

INPUT="$(cat 2>/dev/null || true)"
KEY_RE='"stop_hook_active"[[:space:]]*:'
[[ "$INPUT" =~ $KEY_RE ]] || exit 0

ACTIVE='"stop_hook_active"[[:space:]]*:[[:space:]]*true'
[[ "$INPUT" =~ $ACTIVE ]] && exit 0

DIR="${CLAUDE_PROJECT_DIR:-$PWD}"
CWD_RE='"cwd"[[:space:]]*:[[:space:]]*"([^"]+)"'
[[ "$INPUT" =~ $CWD_RE ]] && DIR="${BASH_REMATCH[1]}"

ROOT="$(git -C "$DIR" rev-parse --show-toplevel 2>/dev/null)" || exit 0
GITDIR="$(git -C "$DIR" rev-parse --absolute-git-dir 2>/dev/null)" || exit 0
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"
BUNDLE="${BUNDLE%/}"

SNAPSHOT=""
while IFS= read -r -d '' entry; do
  code="${entry:0:2}"
  path="${entry:3}"
  origin=""
  case "$code" in R* | C* | *R | *C) read -r -d '' origin ;; esac
  case "$path" in "$BUNDLE"/*) exit 0 ;; esac
  case "$origin" in "$BUNDLE"/*) exit 0 ;; esac
  if [ -f "$ROOT/$path" ]; then
    hash="$(git -C "$ROOT" hash-object -- "$path" 2>/dev/null)"
  else
    hash="absent"
  fi
  SNAPSHOT+="$path $hash"$'\n'
done < <(git -C "$ROOT" status --porcelain -z -uall 2>/dev/null)

[ -n "$SNAPSHOT" ] || exit 0

KEY="$(printf '%s' "$SNAPSHOT" | git hash-object --stdin 2>/dev/null)" || exit 0
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

REASON="living-docs: code changed this turn and no record did. If you made a material choice, write it into the active issue's Decision section with \`living-docs write set $ISSUE section Decision --from -\`, which replaces the whole section, so pass its current bullets plus the new one; promote it to an ADR only when a future reader would pay to rediscover it. If you made no material decision, say \"no material decision\" and stop. Do not create a new record per change."
printf '{"decision":"block","reason":"%s"}\n' "$(printf '%s' "$REASON" | sed 's/\\/\\\\/g; s/"/\\"/g')"
exit 0
