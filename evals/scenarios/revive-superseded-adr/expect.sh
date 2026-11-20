#!/usr/bin/env bash
#
# expect.sh — the bundle is green and no record is in force beside the one that replaces it.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_check_green || eval_fail "living-docs check is red"
while IFS= read -r line; do
	[[ -n "$line" ]] && eval_fail "$line"
done < <(eval_listed_with_successor)
eval_finish
