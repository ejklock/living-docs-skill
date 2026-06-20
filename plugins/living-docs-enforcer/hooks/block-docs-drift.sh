#!/usr/bin/env bash
#
# block-docs-drift.sh — PreToolUse hook: enforce the Living Docs mechanical
# invariants at the git-commit boundary.
#
# Claude Code passes the hook input as JSON on stdin:
#   { "tool_name": "Bash", "tool_input": { "command": "git commit -m ..." } }
#
# Decision contract (matches the repo's existing PreToolUse blockers, e.g.
# block-dangerous-patterns.sh):
#   exit 0 = allow the command  ·  exit 2 = block it (reason printed to stderr)
#
# This hook fires on every Bash call but only ACTS on `git commit`. On a commit
# it runs the bundled lint-docs.sh against the repo's docs bundle and then:
#
#   - SOUND CHECKS — docs that EXIST being malformed / orphan / broken-link /
#     supersede-broken (lint-docs exit 1). lint-docs has a sound oracle for these
#     (the doc is in the tree and demonstrably wrong), so the DEFAULT is to BLOCK
#     the commit. This is the knob: LIVING_DOCS_ENFORCE=block|warn (default block).
#     With LIVING_DOCS_ENFORCE=warn it prints the violations to stderr and ALLOWS.
#     ──► THE BLOCK-VS-WARN DEFAULT IS THE OPEN POLICY DECISION (see README).
#
#   - THE ABSENCE PROXY — a structural change that touches no docs/ file. There is
#     NO sound oracle for "an ADR should exist here" (a hard block is gameable with
#     an empty doc), so this is WARN-ONLY and NEVER blocks, regardless of the knob.
#     Implemented here as a minimal, clearly-marked structural heuristic.
#
# Defensive by construction: if git or lint-docs is missing, if there is no docs
# bundle, or if anything else goes sideways, the hook EXITS CLEANLY (0) with at
# most a warning — it must never wedge the user's commit on its own errors.

# Note: intentionally NOT `set -e` — a non-zero from an internal probe must not
# be reinterpreted as a block. Blocking is only ever an explicit `exit 2`.
set -uo pipefail

# --- enforcement knob -------------------------------------------------------
# block (default) | warn. Anything else is treated as block (fail safe).
ENFORCE="${LIVING_DOCS_ENFORCE:-block}"

# --- bundle root: default ./docs, overridable for non-standard layouts ------
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"

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

# --- sound checks: run the bundled linter -----------------------------------
LINT_OUT="$(bash "$LINT" "$BUNDLE" 2>&1)"
LINT_RC=$?

# exit 2 from the linter = usage / bundle-not-found. That is OUR misconfiguration,
# not the user's docs being wrong — warn and allow, never block.
if [[ "$LINT_RC" -eq 2 ]]; then
	echo "[living-docs-enforcer] lint-docs could not run (exit 2 — usage/bundle error) — commit allowed:" >&2
	printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
	exit 0
fi

# exit 0 = clean. Allow silently.
if [[ "$LINT_RC" -eq 0 ]]; then
	exit 0
fi

# exit 1 = real violations on docs that exist (the sound checks). Apply the knob.
if [[ "$ENFORCE" == "warn" ]]; then
	echo "[living-docs-enforcer] WARN: Living Docs invariant violations (LIVING_DOCS_ENFORCE=warn — commit allowed):" >&2
	printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
	exit 0
fi

# Default / any non-"warn" value: BLOCK.
echo "BLOCKED by living-docs-enforcer: Living Docs invariant violations in ./$BUNDLE." >&2
printf '%s\n' "$LINT_OUT" | sed 's/^/    /' >&2
echo "" >&2
echo "Fix the docs above and re-commit. To downgrade to a warning, set LIVING_DOCS_ENFORCE=warn." >&2
exit 2
