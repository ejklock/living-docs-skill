#!/usr/bin/env bash
#
# repair-without-transcript.sh — a runner that repairs the legacy bundle with the CLI and
# prints no transcript at all, so nothing can be said about how the files were edited.

set -uo pipefail

cat >/dev/null
living-docs manage fmt docs >/dev/null 2>&1
living-docs manage index adr >/dev/null 2>&1
