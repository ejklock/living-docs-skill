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

# A skill that vendors an upstream spec (reference/SPEC.md next to its SKILL.md) tracks that
# spec's own version, not the repo release — otherwise every release mislabels the spec.
spec_version_for() { # spec_version_for <SKILL.md path> -> prints the vendored spec's declared version, empty when there is none
	local skill_md="$1" spec="$1"
	spec="$(dirname "$skill_md")/reference/SPEC.md"
	[[ -e "$spec" ]] || return 0
	sed -nE 's/^\*\*Version[[:space:]]+([^[:space:]]+).*/\1/p' "$spec" | head -1
}

check_spec() { # check_spec <label> <actual> <spec-version>
	if [[ "$2" != "$3" ]]; then
		printf 'MISMATCH: %-40s = %-10s (expected %s from reference/SPEC.md)\n' "$1" "'$2'" "'$3'"
		fail=1
	fi
}

# Detection is deliberately looser than extraction: a file must be caught as "declares a
# version" even when its key is malformed (e.g. a stray space before the colon), so that
# case fails loudly below instead of extraction's strict anchor silently missing it and the
# file being read as if it declared no version at all.
check_versioned_class() { # check_versioned_class <required|optional> <spec-aware|plain> <file>...
	local requirement="$1" mode="$2"
	shift 2
	local f rel v spec_v
	for f in "$@"; do
		rel="${f#"$root"/}"
		if ! grep -qE '^[[:space:]]*version[[:space:]]*:' "$f"; then
			[[ "$requirement" == required ]] && check "$rel" ""
			continue
		fi
		v="$(sed -nE 's/^version:[[:space:]]*"?([^"]+)"?.*/\1/p' "$f" | head -1)"
		if [[ -z "$v" ]]; then
			printf 'MALFORMED: %-40s declares a version key not in canonical form (expected: version: X)\n' "$rel"
			fail=1
			continue
		fi
		spec_v=""
		[[ "$mode" == spec-aware ]] && spec_v="$(spec_version_for "$f")"
		if [[ -n "$spec_v" ]]; then
			check_spec "$rel" "$v" "$spec_v"
		else
			check "$rel" "$v"
		fi
	done
}

check "VERSION" "$file_ver"

# cli/Cargo.toml declares `version = ` in both [package] and (potentially) [dependencies]
# entries (e.g. a dotted-key dependency table), so extraction is scoped to the [package]
# section only — never a bare grep across the whole file.
check_cargo_package_version() { # check_cargo_package_version <file>
	local f="$1" rel scoped v
	rel="${f#"$root"/}"
	if [[ ! -e "$f" ]]; then
		check "$rel" ""
		return
	fi
	scoped="$(sed -nE '/^\[package\]/,/^\[/p' "$f")"
	if ! grep -qE '^version[[:space:]]*=' <<<"$scoped"; then
		check "$rel" ""
		return
	fi
	v="$(sed -nE 's/^version[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' <<<"$scoped" | head -1)"
	if [[ -z "$v" ]]; then
		printf 'MALFORMED: %-40s declares a version key not in canonical form (expected: version = "X")\n' "$rel"
		fail=1
		return
	fi
	check "$rel" "$v"
}

check_cargo_package_version "$root/cli/Cargo.toml"
check_cargo_package_version "$root/mcp/Cargo.toml"

# npm/package.json declares exactly one top-level "version" key (the package has no
# dependencies), so the first match is the package version.
check_npm_package_version() { # check_npm_package_version <file>
	local f="$1" rel v
	rel="${f#"$root"/}"
	v=""
	[[ -e "$f" ]] && v="$(sed -nE 's/^[[:space:]]*"version"[[:space:]]*:[[:space:]]*"([^"]+)".*/\1/p' "$f" | head -1)"
	check "$rel" "$v"
}

check_npm_package_version "$root/npm/package.json"

# The Claude Code plugin declares its own top-level "version" and pins the npm binary its MCP
# server runs (`@ejklock/living-docs@<v>`); both must equal the release version, so a plugin
# can never ship pointing at another binary. A missing file, key or pin reads as empty and fails.
# The "version" read is the one at brace depth 1 (the manifest's own top level), so a nested
# object's "version" never counts and key order does not matter; string contents are blanked
# before braces are counted. Every pin is checked, not only the first.
check_plugin_version() { # check_plugin_version <file>
	local f="$1" rel v pin pins
	rel="${f#"$root"/}"
	v=""
	pins=""
	if [[ -e "$f" ]]; then
		v="$(awk '
			{
				line = $0
				bare = line
				gsub(/"([^"\\]|\\.)*"/, "", bare)
				if (depth == 1 && match(line, /^[ \t]*"version"[ \t]*:[ \t]*"[^"]+"/)) {
					s = substr(line, RSTART, RLENGTH)
					sub(/^[ \t]*"version"[ \t]*:[ \t]*"/, "", s)
					sub(/"$/, "", s)
					print s
					exit
				}
				n = gsub(/[{[]/, "", bare)
				m = gsub(/[}\]]/, "", bare)
				depth += n - m
			}' "$f")"
		pins="$(sed -nE 's/.*"@ejklock\/living-docs@([^"]+)".*/\1/p' "$f")"
	fi
	check "$rel" "$v"
	if [[ -z "$pins" ]]; then
		check "$rel @ejklock/living-docs@ pin" ""
		return
	fi
	while IFS= read -r pin; do
		check "$rel @ejklock/living-docs@ pin" "$pin"
	done <<<"$pins"
}

check_plugin_version "$root/.claude-plugin/plugin.json"

skill_mds=("$root"/skills/*/SKILL.md)
if [[ ! -e "${skill_mds[0]}" ]]; then
	echo "ERROR: no skills/*/SKILL.md files found" >&2
	exit 1
fi
check_versioned_class required spec-aware "${skill_mds[@]}"

if [[ "$fail" -ne 0 ]]; then
	echo "Version check FAILED."
	exit 1
fi
echo "Version OK: $expected"
