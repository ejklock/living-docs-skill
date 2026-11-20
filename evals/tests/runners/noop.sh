#!/usr/bin/env bash
#
# noop.sh — a runner that reads the task and changes nothing.

set -uo pipefail

cat >/dev/null
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "text", text: "Nothing to do."}]}}'
