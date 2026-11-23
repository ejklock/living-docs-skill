#!/usr/bin/env bash
#
# setup.sh — an accepted decision that already answers the question.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Keep the reporting store on PostgreSQL" "Reports live in PostgreSQL because three services write at once." "keep the reporting store on PostgreSQL, because three services write to it at the same time" >/dev/null
eval_seal
