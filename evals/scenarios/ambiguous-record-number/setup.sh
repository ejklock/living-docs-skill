#!/usr/bin/env bash
#
# setup.sh — adr/0001 and issue/0001 both exist; only the issue is meant.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_adr "Store reports in PostgreSQL" "Reports live in PostgreSQL." "store reports in PostgreSQL" >/dev/null
eval_issue "Migrate the reports to PostgreSQL" "Move the reports tables to PostgreSQL." <<'EOF' >/dev/null
Move the reports tables to PostgreSQL.

### Scope

The reports tables and their readers.

### Acceptance

- Every report reads from PostgreSQL.
EOF
eval_seal
