#!/usr/bin/env bash
#
# expect.sh — the issue is closed and its Decision keeps two bullets and gains one.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

issue="$(eval_record_file issues/0001)"
[[ "$(eval_field "$issue" status)" == closed ]] || eval_fail "issue/0001 is not closed"
decision="$(eval_section "$issue" '### Decision')"
grep -q 'streams rows' <<<"$decision" || eval_fail "the streaming bullet is gone from Decision"
grep -q 'Every field is quoted' <<<"$decision" || eval_fail "the quoting bullet is gone from Decision"
grep -q -i 'semicolon' <<<"$decision" || eval_fail "Decision has no bullet about semicolons"
bullets="$(grep -c '^- ' <<<"$decision" || true)"
[[ "$bullets" == 3 ]] || eval_fail "Decision should hold three bullets, found $bullets"
eval_check_green || eval_fail "living-docs check is red"
eval_finish
