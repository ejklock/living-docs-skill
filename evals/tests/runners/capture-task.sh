#!/usr/bin/env bash
#
# capture-task.sh — a runner that saves the task it receives on stdin, byte for byte, to the
# file named by LIVING_DOCS_EVAL_PROBE.

set -uo pipefail

cat >"${LIVING_DOCS_EVAL_PROBE:?}"
