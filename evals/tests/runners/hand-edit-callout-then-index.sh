#!/usr/bin/env bash
#
# hand-edit-callout-then-index.sh — the runner that edits the callout by hand, followed by a
# regeneration of the ADR index with the CLI.

set -uo pipefail

bash "$(dirname "$0")/hand-edit-callout.sh"
living-docs manage index adr >/dev/null 2>&1
