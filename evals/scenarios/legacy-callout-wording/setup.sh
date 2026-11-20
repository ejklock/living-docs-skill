#!/usr/bin/env bash
#
# setup.sh — a superseded ADR and an index written with the earlier callout wording.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Use a message queue" "Work is handed over through a queue." "hand background work over through a message queue" >/dev/null
eval_adr "Use a database table as the queue" "Work is handed over through a table." "hand background work over through a database table" >/dev/null
living-docs write supersede adr/0001 adr/0002 >/dev/null
old="$(eval_record_file adr/0001)"
sed -E -i 's/^(> \*\*SUPERSEDED — do not act on this record\.\*\* Replaced by \[[0-9]+\]\([^)]*\)\.).*/\1 Run `living-docs read` for what is in force./' "$old"
sed -E -i 's/^_History only\. Do not act on these records.*/_History only. Do not act on these records — run `living-docs read` for what is in force._/' "$(dirname "$old")/index.md"
if eval_check_green; then
	echo "setup: the earlier wording still passes check, the scenario would prove nothing" >&2
	exit 1
fi
eval_seal
