#!/usr/bin/env bash
#
# expect.sh — the issue is closed and the decision with the same number is untouched.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

[[ "$(eval_field "$(eval_record_file issues/0001)" status)" == closed ]] || eval_fail "issue/0001 is not closed"
eval_untouched "$(eval_record_file adr/0001)" || eval_fail "adr/0001 changed"
eval_check_green || eval_fail "living-docs check is red"
eval_finish
