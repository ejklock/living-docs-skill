#!/usr/bin/env bash
# update-spec.sh
# Refreshes the vendored OKF specification from its upstream GitHub source.
#
# The skill ships a pinned, verbatim copy of the OKF SPEC.md under
# reference/SPEC.md so the format rules are usable offline and diffable in
# version control. This script re-pulls the upstream copy, overwrites the
# vendored one, and rewrites reference/SPEC.source.md with fresh provenance
# (URL, ref, retrieval time, sha256). Run it, then review the git diff before
# committing — a non-empty diff means upstream OKF changed and the SKILL.md
# rules may need to follow.
#
# Usage:
#   ./update-spec.sh                 # pull from the default ref (main)
#   ./update-spec.sh v0.2            # pull from a specific tag/branch/commit
#   OKF_RAW_URL=<url> ./update-spec.sh   # override the source entirely
#
# Exit 0 = spec fetched (whether or not it changed)
# Exit 1 = fetch failed

set -euo pipefail

# --- Upstream coordinates (single source of truth for where the spec lives) ---
OKF_REPO="GoogleCloudPlatform/knowledge-catalog"
OKF_PATH="okf/SPEC.md"
OKF_REF="${1:-main}"
OKF_RAW_URL="${OKF_RAW_URL:-https://raw.githubusercontent.com/${OKF_REPO}/${OKF_REF}/${OKF_PATH}}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REF_DIR="$(cd "$SCRIPT_DIR/../reference" && pwd)"
SPEC_FILE="$REF_DIR/SPEC.md"
SOURCE_FILE="$REF_DIR/SPEC.source.md"

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

echo "Fetching OKF spec:"
echo "  repo: $OKF_REPO"
echo "  ref:  $OKF_REF"
echo "  url:  $OKF_RAW_URL"

if ! curl -fsSL "$OKF_RAW_URL" -o "$tmp"; then
  echo "ERROR: failed to fetch $OKF_RAW_URL" >&2
  echo "Network access may be restricted in this environment, or the ref does not exist." >&2
  exit 1
fi

if [[ ! -s "$tmp" ]]; then
  echo "ERROR: fetched spec is empty — refusing to overwrite vendored copy." >&2
  exit 1
fi

new_sha="$(sha256sum "$tmp" | awk '{print $1}')"
old_sha=""
[[ -f "$SPEC_FILE" ]] && old_sha="$(sha256sum "$SPEC_FILE" | awk '{print $1}')"

cp "$tmp" "$SPEC_FILE"

cat > "$SOURCE_FILE" <<EOF
# OKF spec — provenance

This directory's \`SPEC.md\` is a verbatim, vendored copy of the upstream
Open Knowledge Format specification. Do not hand-edit \`SPEC.md\`; refresh it
with \`scripts/update-spec.sh\` so this provenance stays accurate.

| Field | Value |
|---|---|
| Source repo | \`${OKF_REPO}\` |
| Source path | \`${OKF_PATH}\` |
| Ref | \`${OKF_REF}\` |
| Raw URL | ${OKF_RAW_URL} |
| Retrieved | $(date -u +%Y-%m-%dT%H:%M:%SZ) |
| sha256 | \`${new_sha}\` |
EOF

echo
if [[ "$new_sha" == "$old_sha" ]]; then
  echo "Spec unchanged (sha256 $new_sha)."
else
  echo "Spec UPDATED."
  echo "  old sha256: ${old_sha:-<none>}"
  echo "  new sha256: $new_sha"
  echo "Review 'git diff skills/okf-knowledge-format/reference/SPEC.md' and update"
  echo "SKILL.md rules if the conformance requirements changed."
fi
