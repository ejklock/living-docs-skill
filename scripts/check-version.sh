#!/usr/bin/env bash
#
# check-version.sh — assert the release version is consistent everywhere it is declared.
#
# The version is necessarily declared in more than one place (the VERSION file and each
# SKILL.md's frontmatter). Duplication drifts — so it is gated: this script is the
# instrument that keeps the copies in agreement ("a constraint without an instrument is a
# vibe"). CI runs it with no argument (internal consistency); the release workflow runs it
# with the git tag (so a forgotten bump fails the release).
#
# Usage:  check-version.sh [EXPECTED]   (default: contents of VERSION)
# Exit:   0 = all agree, 1 = mismatch.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
file_ver="$(tr -d '[:space:]' < "$root/VERSION")"
expected="${1:-$file_ver}"
expected="${expected#v}" # tolerate a leading 'v' from a git tag

fail=0
check() { # check <label> <actual>
	if [[ "$2" != "$expected" ]]; then
		printf 'MISMATCH: %-40s = %-10s (expected %s)\n' "$1" "'$2'" "'$expected'"
		fail=1
	fi
}

check "VERSION" "$file_ver"
for s in living-docs okf-knowledge-format research-artifacts; do
	v="$(grep -E '^version:' "$root/skills/$s/SKILL.md" | head -1 | sed -E 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/')"
	check "skills/$s/SKILL.md" "$v"
done

if [[ "$fail" -ne 0 ]]; then
	echo "Version check FAILED."
	exit 1
fi
echo "Version OK: $expected"
