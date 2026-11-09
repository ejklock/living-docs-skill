#!/usr/bin/env bash
set -euo pipefail
cat >/dev/null
echo "$$" >"$STUB_DIR/judge.pid"
sleep 30 &
echo "$!" >"$STUB_DIR/sleep.pid"
wait
