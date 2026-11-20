#!/usr/bin/env bash
#
# expect.sh — nothing in the bundle changed: the question needed an answer, not a record.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

count="$(find docs/adr -name '[0-9][0-9][0-9][0-9]-*.md' | wc -l | tr -d ' ')"
[[ "$count" == 1 ]] || eval_fail "expected the one ADR, found $count"
eval_untouched docs || eval_fail "the bundle changed"
eval_finish
