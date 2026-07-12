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
LINT_MERMAID="$HERE/../scripts/lint-mermaid.sh"
FIXTURES="$HERE/fixtures"

fail=0

assert_result() { # assert_result <name> <expected_exit> <present|absent> <substring> <actual_exit> <output>
	local name="$1" exp="$2" mode="$3" sub="$4" rc="$5" out="$6"
	local ok=1
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

run() { # run <name> <expected_exit> <present|absent> <substring>
	local name="$1" exp="$2" mode="$3" sub="$4"
	local out rc
	out="$("$LINT" "$FIXTURES/$name/docs" 2>&1)"
	rc=$?
	assert_result "$name" "$exp" "$mode" "$sub" "$rc" "$out"
}

run_mermaid() { # run_mermaid <name> <expected_exit> <present|absent> <substring>
	local name="$1" exp="$2" mode="$3" sub="$4"
	local out rc
	out="$("$LINT_MERMAID" "$FIXTURES/$name" 2>&1)"
	rc=$?
	assert_result "$name" "$exp" "$mode" "$sub" "$rc" "$out"
}

echo "lint-docs hostile fixtures"
echo

# Links (delegated to lychee): fenced code blocks are skipped; titled / angle-bracket /
# bare / reference-style links all resolve; broken links of any form are caught.
run 01-fence-link-clean             0 absent  "broken link"
run 02-fence-link-dirty             1 present "broken link"
run 03-link-forms                   0 absent  "broken link"
run 08-reference-link-broken        1 present "broken link"

# Frontmatter (delegated to yq, real YAML): quotes + inline comment read fine; a block
# scalar is a valid value; a nested key does NOT rescue a missing top-level key.
run 04-frontmatter-quoted-commented 0 absent  "non-empty 'type'"
run 06-block-scalar-ok              0 absent  "non-empty 'type'"
run 05-nested-key-trap              1 present "non-empty 'type'"

# Invariant-4 regression guard — broken superseded_by still fires.
run 07-supersede-broken            1 present "has no matching record"

# Positive parity — the OKF format author's own canonical bundle must pass clean
# (vendored from GoogleCloudPlatform/knowledge-catalog; see the fixture's PROVENANCE.md).
run 09-okf-canonical                0 present "no invariant violations"

# lint-mermaid.sh (delegated to the real Mermaid parser via mermaid-cli): a valid flowchart
# + erDiagram pass clean; a syntactically broken diagram fails with a file:line pointer.
run_mermaid 10-mermaid-valid        0 absent  "FAIL"
run_mermaid 11-mermaid-invalid      1 present "doc.md:"

echo
if ((fail == 0)); then
	echo "All fixtures passed."
	exit 0
else
	echo "Fixture failures."
	exit 1
fi
