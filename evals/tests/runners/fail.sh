#!/usr/bin/env bash
#
# fail.sh — a runner that prints a partial transcript and exits non-zero.

set -uo pipefail

cat >/dev/null
jq -nc '{type: "assistant", message: {role: "assistant", content: [{type: "text", text: "Starting."}]}}'
exit 3
