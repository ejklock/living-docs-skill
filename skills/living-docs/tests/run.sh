#!/usr/bin/env bash
#
# run.sh — hostile/negative fixtures for lint-docs.sh.
#
# The example corpus (examples/linkly/docs) only exercises the happy path, so it can't
# catch regressions in the three fragile parsers (link extraction, link resolution,
# frontmatter reading). Each fixture below asserts the linter's exit code AND that an
# expected violation string is present (or absent) in its output.
#
# Exit: 0 = all fixtures pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
LINT="$HERE/../scripts/lint-docs.sh"
FIXTURES="$HERE/fixtures"

fail=0

run() { # run <name> <expected_exit> <present|absent> <substring>
	local name="$1" exp="$2" mode="$3" sub="$4"
	local out rc ok=1
	out="$("$LINT" "$FIXTURES/$name/docs" 2>&1)"
	rc=$?
	[[ "$rc" == "$exp" ]] || ok=0
	if [[ "$mode" == "present" ]]; then
		grep -qF -- "$sub" <<<"$out" || ok=0
	else
		grep -qF -- "$sub" <<<"$out" && ok=0
	fi
	if ((ok == 1)); then
		printf '  ok    %s\n' "$name"
	else
		printf '  FAIL  %s — exit %s (expected %s), expected %s: "%s"\n' \
			"$name" "$rc" "$exp" "$mode" "$sub"
		printf '%s\n' "$out" | sed 's/^/          | /'
		fail=1
	fi
}

echo "lint-docs hostile fixtures"
echo

# Bug 1 — fenced code blocks are skipped.
run 01-fence-link-clean             0 absent  "broken link"
run 02-fence-link-dirty             1 present "broken link"

# Bug 2 — titled / angle-bracket / bare links all resolve.
run 03-link-forms                   0 absent  "broken link"

# Bug 3 — frontmatter reader: quotes + inline comment read fine;
#         nested key does not rescue a missing top-level key; block scalar is rejected.
run 04-frontmatter-quoted-commented 0 absent  "non-empty 'type'"
run 05-nested-key-trap              1 present "non-empty 'type'"
run 06-block-scalar                1 present "non-empty 'type'"

# Invariant-4 regression guard — broken superseded_by still fires.
run 07-supersede-broken            1 present "has no matching record"

echo
if ((fail == 0)); then
	echo "All fixtures passed."
	exit 0
else
	echo "Fixture failures."
	exit 1
fi
