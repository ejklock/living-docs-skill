#!/usr/bin/env bash
# install.sh — copy the Living Docs skill bundle into an agent skills directory.
#
# Usage:
#   ./install.sh                      # install to ~/.claude/skills
#   ./install.sh /path/to/skills      # install to a custom skills directory
#   SKILLS_DIR=/path ./install.sh     # same, via env var
#
# Idempotent: re-running overwrites the three skill directories in place.
# Only the skills are installed; references/ and docs stay in this repo.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST="${1:-${SKILLS_DIR:-$HOME/.claude/skills}}"

SKILLS=(living-docs okf-knowledge-format research-artifacts)

mkdir -p "$DEST"

for skill in "${SKILLS[@]}"; do
  src="$SCRIPT_DIR/skills/$skill"
  if [[ ! -d "$src" ]]; then
    echo "ERROR: missing skill source: $src" >&2
    exit 1
  fi
  rm -rf "${DEST:?}/$skill"
  cp -R "$src" "$DEST/$skill"
  echo "installed: $skill -> $DEST/$skill"
done

echo
echo "Done. Three skills installed to $DEST"
echo "Restart your agent session so the skills are picked up."
