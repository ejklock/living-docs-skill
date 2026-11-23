#!/usr/bin/env bash
#
# hand-edit-callout.sh — a runner that reports editing the callout of the superseded record by
# hand (an Edit line in its transcript) while the tree itself is repaired with manage fmt.

set -uo pipefail

cat >/dev/null
record="$(find docs -name '0001-*.md' -path '*/adr/*' | head -n 1)"
callout='> **SUPERSEDED — do not act on this record.** Replaced by [0002](0002-x.md).'
jq -nc --arg path "$record" --arg old "$callout Run \`living-docs read\` for what is in force." --arg new "$callout" \
	'{type: "assistant", message: {role: "assistant", content: [{type: "tool_use", id: "edit_1", name: "Edit", input: {file_path: $path, old_string: $old, new_string: $new}}]}}'
living-docs manage fmt docs >/dev/null 2>&1
