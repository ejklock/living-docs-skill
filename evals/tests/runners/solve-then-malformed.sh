#!/usr/bin/env bash
#
# solve-then-malformed.sh — the scripted solver followed by transcript lines that are valid
# JSON in the wrong shape: a tool call with no id, a numeric id, a string input, an array.

set -uo pipefail

bash "$(dirname "$0")/solve.sh"
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "tool_use", name: "Bash", input: {command: "living-docs check"}}]}}'
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "tool_use", id: 7, name: "Edit", input: {file_path: "docs/x.md"}}]}}'
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "tool_use", id: "s", name: "Bash", input: "living-docs check"}]}}'
printf '[1, 2, 3]\n'
