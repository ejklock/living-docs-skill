#!/usr/bin/env bash
#
# expect.sh — the record names the new path and no record names the old one.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

if grep -rq 'src/cache\.rs' docs; then
	eval_fail "a record still names src/cache.rs"
fi
grep -q 'src/store/cache\.rs' "$(eval_record_file adr/0001)" || eval_fail "adr/0001 does not name src/store/cache.rs"
[[ "$(eval_field "$(eval_record_file adr/0001)" status)" == Accepted ]] || eval_fail "adr/0001 is no longer Accepted"
eval_check_green || eval_fail "living-docs check is red"
eval_finish
