#!/usr/bin/env bash
#
# setup.sh — an accepted decision to keep sessions in Redis.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Keep sessions in Redis" "Sessions live in a shared Redis cluster." "keep every user session in a shared Redis cluster" >/dev/null
eval_seal
