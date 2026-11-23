#!/usr/bin/env bash
#
# setup.sh — a bundle with one unrelated accepted decision; the task is a new decision to record.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Emit structured JSON logs" "Every service logs one JSON object per line." "emit one JSON object per log line from every service" >/dev/null
eval_seal
