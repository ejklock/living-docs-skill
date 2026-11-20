#!/usr/bin/env bash
#
# expect.sh — one new decision about PostgreSQL exists, and the unrelated one is untouched.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

count="$(find docs/adr -name '[0-9][0-9][0-9][0-9]-*.md' | wc -l | tr -d ' ')"
[[ "$count" == 2 ]] || eval_fail "expected two ADR files, found $count"
grep -q -i 'postgres' "$(eval_record_file adr/0002)" 2>/dev/null || eval_fail "no adr/0002 that mentions PostgreSQL"
eval_untouched "$(eval_record_file adr/0001)" || eval_fail "adr/0001 changed"
eval_finish
