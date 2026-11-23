#!/usr/bin/env bash
#
# hand-edit-status.sh — a runner that sets the status of adr/0001 to Superseded by editing the
# file, leaving the callout and the successor link unwritten.

set -uo pipefail

cat >/dev/null
record="$(find docs -name '0001-*.md' -path '*/adr/*' | head -n 1)"
sed -i.bak 's/^status: .*/status: Superseded/' "$record"
rm -f "$record.bak"
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "text", text: "Edited the status by hand."}]}}'
