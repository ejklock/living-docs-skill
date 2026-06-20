#!/usr/bin/env bash
#
# block-docs-drift.sh — PreToolUse hook: enforce the Living Docs mechanical
# invariants at the git-commit boundary, DIFF-AWARE.
#
# Claude Code passes the hook input as JSON on stdin:
#   { "tool_name": "Bash", "tool_input": { "command": "git commit -m ..." } }
#
# Decision contract (matches the repo's existing PreToolUse blockers, e.g.
# block-dangerous-patterns.sh):
#   exit 0 = allow the command  ·  exit 2 = block it (reason printed to stderr)
#
# This hook fires on every Bash call but only ACTS on `git commit`. On a commit
# it runs the bundled linter in DIFF-AWARE RATCHET mode against HEAD and then:
#
#   - SOUND CHECKS — docs that EXIST being malformed / orphan / broken-link /
#     supersede-broken. The hook runs `lint-docs.sh --ratchet HEAD <bundle>`, so
#     it blocks ONLY violations this commit INTRODUCES and grandfathers any
#     pre-existing legacy debt (ratchet exit 1 = NEW violations). lint-docs has a
#     sound oracle for these (the doc is in the tree and demonstrably wrong), and
#     because the ratchet never punishes you for legacy debt you did not touch,
#     block-by-default is adoption-safe — so the SETTLED default is to BLOCK.
#     The one knob: LIVING_DOCS_ENFORCE=block|warn (default block).
#     With LIVING_DOCS_ENFORCE=warn it prints the NEW violations and ALLOWS.
#
#   - THE ABSENCE PROXY — a structural change that touches no docs/ file. There is
#     NO sound oracle for "an ADR should exist here" (a hard block is gameable with
#     an empty doc), so this is WARN-ONLY and NEVER blocks, regardless of the knob.
#     Implemented here as a minimal, clearly-marked structural heuristic.
#
# Defensive by construction: if git or lint-docs is missing, if there is no docs
# bundle, or if anything else goes sideways, the hook EXITS CLEANLY (0) with at
# most a warning — it must never wedge the user's commit on its own errors.
#
# This is the Claude-Code IN-LOOP layer. The harness-agnostic siblings —
# enforcement/pre-commit.sh (git hook) and the CI Action — call the SAME
# `lint-docs.sh --ratchet` CLI at the same boundary for non-Claude commits.

# Note: intentionally NOT `set -e` — a non-zero from an internal probe must not
# be reinterpreted as a block. Blocking is only ever an explicit `exit 2`.
set -uo pipefail

# git exports ambient vars into hooks (GIT_INDEX_FILE / GIT_DIR / GIT_WORK_TREE),
# often as paths relative to the commit's cwd. A PreToolUse hook firing DURING a
# `git commit` inherits the same ambient env, and those vars corrupt the
# `git worktree add` the --ratchet mode uses to materialize the HEAD baseline.
# Clear them so the linter's own git calls operate on the repo cleanly (same fix
# as enforcement/pre-commit.sh).
unset GIT_INDEX_FILE GIT_DIR GIT_WORK_TREE GIT_PREFIX

# --- enforcement knob -------------------------------------------------------
# block (default, the settled policy) | warn. Anything else is treated as block
# (fail safe).
ENFORCE="${LIVING_DOCS_ENFORCE:-block}"

# --- bundle root: default ./docs, overridable for non-standard layouts ------
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"

# --- the diff-aware baseline ------------------------------------------------
# Compare the bundle you are about to commit against the last commit, so only
# violations YOU are introducing can ever block. Pre-existing debt is grandfathered.
BASELINE_REF="HEAD"

# --- read + parse the hook payload ------------------------------------------
INPUT="$(cat)"

if command -v jq >/dev/null 2>&1; then
	TOOL_NAME="$(printf '%s' "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)"
	COMMAND="$(printf '%s' "$INPUT" | jq -r '.tool_input.command // empty' 2>/dev/null)"
else
	TOOL_NAME="$(printf '%s' "$INPUT" \
		| grep -o '"tool_name"[[:space:]]*:[[:space:]]*"[^"]*"' \
		| sed 's/.*"tool_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')"
	COMMAND_RAW="$(printf '%s' "$INPUT" \
		| sed 's/.*"command"[[:space:]]*:[[:space:]]*"\(.*\)"}}.*/\1/')"
	COMMAND="$(printf '%s' "$COMMAND_RAW" \
		| sed 's/\\"/"/g' \
		| sed 's/\\n/\n/g' \
		| sed 's/\\t/\t/g' \
		| sed 's/\\\\/\\/g')"
fi

# Only act on Bash, and only on a git commit. Everything else: allow.
[[ "$TOOL_NAME" != "Bash" ]] && exit 0
printf '%s' "$COMMAND" | grep -qE 'git([[:space:]]+-[^[:space:]]+)*[[:space:]]+commit([[:space:]]|$)' || exit 0

# --- locate the bundled linter ----------------------------------------------
# CLAUDE_PLUGIN_ROOT is exported by Claude Code into hook processes. Fall back to
# the script's own location so the hook is also runnable standalone (e.g. in CI).
PLUGIN_ROOT="${CLAUDE_PLUGIN_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
LINT="$PLUGIN_ROOT/skills/living-docs/scripts/lint-docs.sh"

# --- defensive guards: never wedge the commit on our own missing deps -------
if ! command -v git >/dev/null 2>&1; then
	echo "[living-docs-enforcer] git not found — skipping docs lint (commit allowed)." >&2
	exit 0
fi
if [[ ! -f "$LINT" ]]; then
	echo "[living-docs-enforcer] lint-docs.sh not found at $LINT — skipping docs lint (commit allowed)." >&2
	exit 0
fi
if [[ ! -d "$BUNDLE" ]]; then
	# No docs bundle at all. This is the absence case at the extreme; warn only.
	echo "[living-docs-enforcer] no docs bundle at ./$BUNDLE — nothing to lint (commit allowed)." >&2
	exit 0
fi

# --- absence proxy (WARN-ONLY, never blocks) --------------------------------
# Heuristic: if the staged change touches code/structure but NOT a single docs/
# file, surface a gentle reminder. There is no sound oracle for "an ADR belongs
# here", so this can never be a block — a hard block would just be satisfied with
# an empty placeholder doc (Goodhart). Best-effort; silent on any git error.
STAGED="$(git diff --cached --name-only 2>/dev/null || true)"
if [[ -n "$STAGED" ]]; then
	if ! printf '%s\n' "$STAGED" | grep -qE "(^|/)${BUNDLE}/"; then
		if printf '%s\n' "$STAGED" | grep -qE '\.(ts|tsx|js|jsx|py|go|rb|php|java|kt|rs|c|cc|cpp|h|hpp|cs|swift|sql)$'; then
			echo "[living-docs-enforcer] reminder: this commit changes code but touches no ./$BUNDLE file." >&2
			echo "                       if it is a structural change (new module, moved files, schema/flow change)," >&2
			echo "                       Living Docs invariant 5 wants the doc + diagram updated in the same change." >&2
			echo "                       (advisory only — not a sound check, never blocks)" >&2
		fi
	fi
fi

# --- sound checks: run the bundled linter in DIFF-AWARE RATCHET mode ---------
# `--ratchet HEAD` => exit 0 on clean OR pre-existing-only debt, exit 1 on NEW
# violations this commit introduces, exit 2 on usage / bundle error.
LINT_OUT="$(bash "$LINT" --ratchet "$BASELINE_REF" "$BUNDLE" 2>&1)"
LINT_RC=$?

# exit 2 from the linter = usage / bundle-not-found / no-git-baseline. That is OUR
# misconfiguration, not the user's docs being wrong — warn and allow, never block.
if [[ "$LINT_RC" -eq 2 ]]; then
	echo "[living-docs-enforcer] lint-docs --ratchet could not run (exit 2 — usage/bundle/baseline error) — commit allowed:" >&2
	printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
	exit 0
fi

# exit 0 = clean, or only pre-existing debt (grandfathered). Allow silently.
if [[ "$LINT_RC" -eq 0 ]]; then
	exit 0
fi

# exit 1 = NEW violations this commit introduces (the sound checks). Apply the knob.
if [[ "$ENFORCE" == "warn" ]]; then
	echo "[living-docs-enforcer] WARN: this commit introduces NEW Living Docs invariant violation(s) (LIVING_DOCS_ENFORCE=warn — commit allowed):" >&2
	printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
	exit 0
fi

# Default / any non-"warn" value: BLOCK.
echo "BLOCKED by living-docs-enforcer: this commit introduces NEW Living Docs invariant violation(s) in ./$BUNDLE." >&2
echo "(Pre-existing legacy debt is grandfathered by the diff-aware ratchet — only NEW violations block.)" >&2
printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
echo "" >&2
echo "Fix the docs above and re-commit. To downgrade to a warning, set LIVING_DOCS_ENFORCE=warn." >&2
exit 2
