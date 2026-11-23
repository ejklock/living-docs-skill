#!/usr/bin/env bash
#
# expect.sh — adr/0001 is retired in favour of a new ADR that is in force.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

old="$(eval_record_file adr/0001)"
[[ "$(eval_field "$old" status)" == Superseded ]] || eval_fail "adr/0001 is not Superseded"
successor="$(eval_field "$old" superseded_by)"
[[ -n "$successor" ]] || eval_fail "adr/0001 names no successor"
new="$(eval_record_file "adr/${successor:-0000}")"
[[ -n "$new" && -f "$new" ]] || eval_fail "the successor ADR $successor does not exist"
[[ "$successor" != 0001 ]] || eval_fail "adr/0001 supersedes itself"
eval_check_green || eval_fail "living-docs check is red"
living-docs read records --json 2>/dev/null | jq -e --argjson n "${successor:-0}" '[.records[] | select(.type == "ADR") | .number] | (index($n) != null) and (index(1) == null)' >/dev/null ||
	eval_fail "read records does not show only the successor"
eval_finish
