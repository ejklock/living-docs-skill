#!/usr/bin/env bash
#
# run-ratchet.sh — corpus for the DIFF-AWARE RATCHET mode of lint-docs.sh.
#
# The default-mode corpus (run.sh) proves the linter catches each whole-bundle
# violation. This corpus proves the *ratchet* layer on top of it: that only NEW
# violations (present now, absent at the baseline ref) block, while pre-existing
# debt is grandfathered — the same diff-aware semantics the repo's other gates use.
#
# Each case spins up a THROWAWAY git repo (init → commit a baseline → mutate the
# working tree), so the cases exercise the real `git worktree add` baseline
# materialization end-to-end. A local git identity is set inside each temp repo so
# commits work in CI.
#
# Cases:
#   new-blocks        clean committed bundle, introduce ONE violation → exit 1, names it
#   preexisting-ok    bundle with a committed violation, make an unrelated clean change
#                     → exit 0 (legacy debt is not held against the change)
#   fix-passes        remove a pre-existing violation → exit 0
#   baseline-absent   --ratchet against a non-existent ref → baseline empty, current
#                     violation counts as NEW → exit 1 (fail-closed)
#
# Exit: 0 = every case behaved as asserted · 1 = at least one case failed.

set -uo pipefail

if [[ -z "${BASH_VERSINFO:-}" || "${BASH_VERSINFO[0]}" -lt 4 ]]; then
	echo "run-ratchet.sh requires bash 4+ (you have ${BASH_VERSION:-unknown})." >&2
	exit 2
fi

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
LINT="$REPO_ROOT/skills/living-docs/scripts/lint-docs.sh"

if [[ ! -x "$LINT" ]]; then
	echo "run-ratchet.sh: linter not found or not executable: $LINT" >&2
	exit 2
fi

PASS=0
FAIL=0
WORKDIRS=()

cleanup() {
	local d
	for d in "${WORKDIRS[@]:-}"; do
		[[ -n "$d" && -d "$d" ]] && rm -rf "$d"
	done
}
trap cleanup EXIT

# new_repo → prints the path to a fresh git repo with identity configured
new_repo() {
	local d
	d="$(mktemp -d "${TMPDIR:-/tmp}/ldr.XXXXXX")"
	WORKDIRS+=("$d")
	git -C "$d" init -q
	git -C "$d" config user.email "ratchet-test@example.com"
	git -C "$d" config user.name "Ratchet Test"
	# discourage a stray global hooksPath from interfering with these temp commits
	git -C "$d" config core.hooksPath /dev/null 2>/dev/null || true
	printf '%s\n' "$d"
}

# write a minimal CLEAN bundle under <repo>/docs
seed_clean_bundle() {
	local repo="$1"
	mkdir -p "$repo/docs/adr"
	cat >"$repo/docs/index.md" <<'EOF'
---
okf_version: 0.1
---
# Docs
- [ADR](/adr/)
EOF
	cat >"$repo/docs/adr/index.md" <<'EOF'
# ADR
- [0001](0001-first.md)
EOF
	cat >"$repo/docs/adr/0001-first.md" <<'EOF'
---
type: adr
---
# First decision
EOF
}

# assert <name> <expected-exit> <output> <actual-exit> [substring]
assert() {
	local name="$1" want_exit="$2" out="$3" got_exit="$4" want_grep="${5:-}"
	local ok=1 reason=""
	if [[ "$got_exit" -ne "$want_exit" ]]; then
		ok=0
		reason="exit $got_exit (wanted $want_exit)"
	fi
	if [[ -n "$want_grep" ]] && ! grep -qF -- "$want_grep" <<<"$out"; then
		ok=0
		reason="${reason:+$reason; }missing substring: '$want_grep'"
	fi
	if [[ "$ok" -eq 1 ]]; then
		printf 'PASS  %-24s exit=%s\n' "$name" "$got_exit"
		PASS=$((PASS + 1))
	else
		printf 'FAIL  %-24s %s\n' "$name" "$reason"
		printf '%s\n' "$out" | sed 's/^/        | /'
		FAIL=$((FAIL + 1))
	fi
}

echo "=== lint-docs.sh ratchet corpus ==="
echo

# Cases are functions run in the current shell (not subshells) so the PASS/FAIL
# counters aggregate into the final summary.

run_new_blocks() {
	local repo out exit_out
	repo="$(new_repo)"
	seed_clean_bundle "$repo"
	git -C "$repo" add -A
	git -C "$repo" commit -qm "clean baseline"
	echo '- [dead](this-file-does-not-exist.md)' >>"$repo/docs/adr/0001-first.md"
	out="$(cd "$repo" && "$LINT" --ratchet HEAD docs 2>&1)"
	exit_out=$?
	assert "new-blocks" 1 "$out" "$exit_out" "this-file-does-not-exist.md"
	# the NEW marker must be present and the fail summary must mention 1 new violation
	assert "new-blocks-marked" 1 "$out" "$exit_out" "NEW violations introduced"
}

run_preexisting_ok() {
	local repo out exit_out
	repo="$(new_repo)"
	seed_clean_bundle "$repo"
	# commit a bundle that ALREADY carries a broken link (pre-existing debt)
	echo '- [legacy-dead](legacy-missing.md)' >>"$repo/docs/adr/0001-first.md"
	git -C "$repo" add -A
	git -C "$repo" commit -qm "baseline with pre-existing debt"
	# make an UNRELATED, clean change in the working tree
	echo '<!-- unrelated clean edit -->' >>"$repo/docs/adr/index.md"
	out="$(cd "$repo" && "$LINT" --ratchet HEAD docs 2>&1)"
	exit_out=$?
	assert "preexisting-ok" 0 "$out" "$exit_out" "no new violations"
	assert "preexisting-grandfathered" 0 "$out" "$exit_out" "grandfathered"
}

run_fix_passes() {
	local repo out exit_out
	repo="$(new_repo)"
	seed_clean_bundle "$repo"
	echo '- [legacy-dead](legacy-missing.md)' >>"$repo/docs/adr/0001-first.md"
	git -C "$repo" add -A
	git -C "$repo" commit -qm "baseline with pre-existing debt"
	# FIX the pre-existing violation in the working tree
	cat >"$repo/docs/adr/0001-first.md" <<'EOF'
---
type: adr
---
# First decision
EOF
	out="$(cd "$repo" && "$LINT" --ratchet HEAD docs 2>&1)"
	exit_out=$?
	assert "fix-passes" 0 "$out" "$exit_out" "no new violations"
}

run_baseline_absent() {
	local repo out exit_out
	repo="$(new_repo)"
	seed_clean_bundle "$repo"
	git -C "$repo" add -A
	git -C "$repo" commit -qm "clean baseline"
	# introduce a violation, but ratchet against a ref that does not exist:
	# baseline is treated as empty → the violation is NEW → exit 1 (fail-closed)
	echo '- [dead](nope.md)' >>"$repo/docs/adr/0001-first.md"
	out="$(cd "$repo" && "$LINT" --ratchet refs/heads/does-not-exist docs 2>&1)"
	exit_out=$?
	assert "baseline-absent" 1 "$out" "$exit_out" "treated as empty"
}

run_new_blocks
run_preexisting_ok
run_fix_passes
run_baseline_absent

echo
echo "=== summary: $PASS passed, $FAIL failed ==="
[[ "$FAIL" -eq 0 ]]
