#!/usr/bin/env bash
#
# setup.sh — adr/0001 superseded by adr/0002.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Store reports in PostgreSQL" "Reports live in PostgreSQL." "store reports in PostgreSQL" >/dev/null
eval_adr "Store reports in SQLite" "Reports live in SQLite." "store reports in SQLite" >/dev/null
living-docs write supersede adr/0001 adr/0002 >/dev/null
eval_seal
