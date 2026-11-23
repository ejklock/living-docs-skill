#!/usr/bin/env bash
#
# expect.sh — the bundle is green and the earlier wording is gone.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_check_green || eval_fail "living-docs check is red"
if grep -rq 'living-docs read`' docs/adr; then
	eval_fail "a record or index still carries the earlier wording"
fi
pending="$(living-docs manage fmt --check --json docs 2>/dev/null | jq -r '.pending | length')"
[[ "$pending" == 0 ]] || eval_fail "manage fmt still has $pending file(s) pending"
usage="${LIVING_DOCS_EVAL_USAGE:-}"
if ! jq -e '.transcript_recognized == true and (.owned_field_hand_edits | type) == "number"' <<<"$usage" >/dev/null 2>&1; then
	eval_fail "hand edits are not verifiable: the transcript could not be read"
else
	hand_edits="$(jq -r '.owned_field_hand_edits' <<<"$usage")"
	[[ "$hand_edits" == 0 ]] || eval_fail "$hand_edits owned text edit(s) were made by hand: the CLI rewrites a callout and an index"
fi
eval_finish
