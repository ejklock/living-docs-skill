#!/usr/bin/env bash
#
# setup.sh — an open issue whose Decision section holds two bullets.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
eval_issue "Add CSV export to the reports page" "Let a user download a report as a CSV file." <<'EOF' >/dev/null
Let a user download the current report as a CSV file.

### Scope

A download button on the reports page.

### Decision

- The export streams rows instead of building the file in memory. Not taken: a background job.
- Every field is quoted, so commas inside text never break a row.

### Acceptance

- A report of ten thousand rows downloads without the server running out of memory.
EOF
eval_seal
