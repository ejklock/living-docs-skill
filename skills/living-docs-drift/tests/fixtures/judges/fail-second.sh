#!/usr/bin/env bash
set -euo pipefail
cat >/dev/null
n=$(find "$STUB_DIR" -name 'call-*' | wc -l | tr -d ' ')
touch "$STUB_DIR/call-$((n + 1))"
if [[ "$n" -ge 1 ]]; then
  echo "judge exploded" >&2
  exit 1
fi
printf '%s\n' '{"schema_version":1,"verdict":"no_drift","findings":[]}'
