#!/usr/bin/env bash
#
# pre-commit.sh — git pre-commit hook: block a commit that introduces NEW
# Living Docs invariant violations, while grandfathering pre-existing debt.
#
# It is a THIN trigger: it does not reimplement any check. It calls the single
# enforcement CLI — `lint-docs.sh --ratchet HEAD` — against the project's docs
# bundle. That one CLI, enforced here at the git boundary (and in CI via the
# reusable Action), is HARNESS-AGNOSTIC: it covers Claude Code, Pi, OpenCode,
# Cursor, Copilot AND plain human commits with one mechanism, instead of a
# separate per-harness plugin. (The per-harness plugin is the optional in-loop
# layer that calls this SAME CLI; it is not built here.)
#
# Baseline = HEAD: the ratchet compares the bundle you are about to commit
# against the last commit, so only violations YOU are introducing block.
#
# Bypassable: `git commit --no-verify` skips all pre-commit hooks (inherent to
# git). The unbypassable backstop is the CI Action.
#
# Defensive: if lint-docs.sh or git is missing, or this is not a git repo, the
# hook exits 0 (it never wedges a commit because tooling is absent).
#
# --- configuration ---------------------------------------------------------
# DOCS_BUNDLE  the docs bundle to lint (default: docs). Override via env, e.g.
#              DOCS_BUNDLE=documentation/docs in your hook wrapper.
# LINT_DOCS    path to lint-docs.sh. Auto-discovered if unset (see below).

set -uo pipefail

# git exports ambient vars into hooks (GIT_INDEX_FILE / GIT_DIR / GIT_WORK_TREE),
# often as paths relative to the commit's cwd. They corrupt the `git worktree add`
# the ratchet uses to materialize the baseline. Clear them so the linter's own git
# calls operate on the repo cleanly.
unset GIT_INDEX_FILE GIT_DIR GIT_WORK_TREE GIT_PREFIX

DOCS_BUNDLE="${DOCS_BUNDLE:-docs}"

# Resolve the repo root; bail cleanly if not in a git repo.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [[ -z "$REPO_ROOT" ]]; then
	exit 0
fi

# Locate lint-docs.sh: explicit env wins, then a few conventional locations.
find_lint() {
	if [[ -n "${LINT_DOCS:-}" && -x "$LINT_DOCS" ]]; then
		printf '%s\n' "$LINT_DOCS"
		return 0
	fi
	local c
	for c in \
		"$REPO_ROOT/skills/living-docs/scripts/lint-docs.sh" \
		"$REPO_ROOT/scripts/lint-docs.sh" \
		"$REPO_ROOT/.claude/skills/living-docs/scripts/lint-docs.sh"; do
		if [[ -x "$c" ]]; then
			printf '%s\n' "$c"
			return 0
		fi
	done
	# fall back to PATH
	if command -v lint-docs.sh >/dev/null 2>&1; then
		command -v lint-docs.sh
		return 0
	fi
	return 1
}

LINT="$(find_lint || true)"
if [[ -z "$LINT" ]]; then
	echo "living-docs pre-commit: lint-docs.sh not found — skipping (set LINT_DOCS to enforce)." >&2
	exit 0
fi

# No docs bundle in this repo → nothing to enforce.
if [[ ! -d "$REPO_ROOT/$DOCS_BUNDLE" ]]; then
	exit 0
fi

# Run the ratchet against HEAD. Exit 1 only on NEW violations.
if ! (cd "$REPO_ROOT" && "$LINT" --ratchet HEAD "$DOCS_BUNDLE"); then
	echo >&2
	echo "living-docs pre-commit: this commit introduces NEW docs invariant violation(s)." >&2
	echo "  Fix them, or bypass intentionally with:  git commit --no-verify" >&2
	exit 1
fi

exit 0
