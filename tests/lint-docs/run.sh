#!/usr/bin/env bash
#
# run.sh — parity/fixture corpus runner for lint-docs.sh.
#
# This is the parity test that makes the linter trustworthy (the "harden" floor):
# it exercises the tool against a curated corpus so we know it (a) stays silent on a
# clean bundle and (b) catches each mechanical violation it claims to catch — one
# violation per dirty fixture, in the spirit of an arch-gate parity test (clean
# partition passes silently; dirty partition each caught 1/1).
#
# Layout:
#   fixtures/clean/...        a clean bundle → asserts exit 0   (also runs the shipped
#                             examples/linkly/docs as a second clean assertion)
#   fixtures/<name>/...       a dirty bundle, one violation     → asserts exit 1 + msg
#   expect/<name>.exit        expected exit code
#   expect/<name>.grep        substring that must appear in the linter output
#
# Exit: 0 = every case behaved as asserted · 1 = at least one case failed.

set -uo pipefail

if [[ -z "${BASH_VERSINFO:-}" || "${BASH_VERSINFO[0]}" -lt 4 ]]; then
	echo "run.sh requires bash 4+ (you have ${BASH_VERSION:-unknown})." >&2
	exit 2
fi

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
LINT="$REPO_ROOT/skills/living-docs/scripts/lint-docs.sh"
FIXTURES="$HERE/fixtures"
EXPECT="$HERE/expect"

if [[ ! -x "$LINT" ]]; then
	echo "run.sh: linter not found or not executable: $LINT" >&2
	exit 2
fi

PASS=0
FAIL=0

# run_case <case-name> <bundle-dir> <expected-exit> [expected-substring]
run_case() {
	local name="$1" bundle="$2" want_exit="$3" want_grep="${4:-}"
	local out got_exit ok=1 reason=""

	out="$("$LINT" "$bundle" 2>&1)"
	got_exit=$?

	if [[ "$got_exit" -ne "$want_exit" ]]; then
		ok=0
		reason="exit $got_exit (wanted $want_exit)"
	fi
	if [[ -n "$want_grep" ]] && ! grep -qF -- "$want_grep" <<<"$out"; then
		ok=0
		reason="${reason:+$reason; }missing substring: '$want_grep'"
	fi

	if [[ "$ok" -eq 1 ]]; then
		printf 'PASS  %-32s exit=%s\n' "$name" "$got_exit"
		PASS=$((PASS + 1))
	else
		printf 'FAIL  %-32s %s\n' "$name" "$reason"
		printf '%s\n' "$out" | sed 's/^/        | /'
		FAIL=$((FAIL + 1))
	fi
}

echo "=== lint-docs.sh fixture corpus ==="
echo

# --- clean partition: must exit 0, silent on violations ---------------------

# The shipped worked example is the canonical clean bundle.
run_case "examples/linkly (shipped)" "$REPO_ROOT/examples/linkly/docs" 0
# A second, minimal clean bundle proves a hand-rolled clean corpus also passes.
run_case "clean (minimal)" "$FIXTURES/clean/docs" 0

echo

# --- dirty partition: one violation per fixture, must exit 1 + emit message --

for dir in "$FIXTURES"/dirty-*/; do
	[[ -d "$dir" ]] || continue
	name="$(basename "$dir")"
	bundle="$dir/docs"
	exit_file="$EXPECT/$name.exit"
	grep_file="$EXPECT/$name.grep"
	want_exit=1
	[[ -f "$exit_file" ]] && want_exit="$(cat "$exit_file")"
	want_grep=""
	[[ -f "$grep_file" ]] && want_grep="$(cat "$grep_file")"
	run_case "$name" "$bundle" "$want_exit" "$want_grep"
done

echo
echo "=== summary: $PASS passed, $FAIL failed ==="
[[ "$FAIL" -eq 0 ]]
