#!/usr/bin/env bash
set -euo pipefail
n=$(find "$STUB_DIR" -name '*.in' | wc -l | tr -d ' ')
cat >"$STUB_DIR/$((n + 1)).in"
printf '%s\n' '{"schema_version":1,"verdict":"no_drift","findings":[]}'
