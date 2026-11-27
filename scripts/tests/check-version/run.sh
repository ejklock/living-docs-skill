#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/check-version.sh.
#
# check-version.sh derives its `root` from its own location (dirname "$0"/..), so each case
# builds a synthetic repo under mktemp -d and runs a COPY of the real script from
# $TMP/<case>/scripts/check-version.sh against that synthetic tree. No network access.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT_SRC="$HERE/../../check-version.sh"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

fail=0

write_frontmatter() { # write_frontmatter <file> <version-line-or-empty>
  local file="$1" line="$2"
  mkdir -p "$(dirname "$file")"
  {
    echo "---"
    echo "title: fixture"
    [[ -n "$line" ]] && echo "$line"
    echo "---"
    echo "# fixture"
  } >"$file"
}

write_cargo() { # write_cargo <dir> <package-version> [<mcp-package-version>] -> realistic layout: [package] first, inline deps after
  local crate version
  for crate in cli mcp; do
    version="$2"
    [[ "$crate" == mcp && -n "${3:-}" ]] && version="$3"
    mkdir -p "$1/$crate"
    cat >"$1/$crate/Cargo.toml" <<TOML
[package]
name = "fixture-$crate"
version = "$version"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
TOML
  done
}

write_cargo_dep_before_package() { # write_cargo_dep_before_package <dir> <package-version> <dependency-version>
  mkdir -p "$1/cli"
  cat >"$1/cli/Cargo.toml" <<TOML
[dependencies]
version = "$3"

[package]
name = "fixture"
version = "$2"
edition = "2021"
TOML
}

set_all_versions() { # set_all_versions <dir> <version>
  local dir="$1" version="$2"
  printf '%s' "$version" >"$dir/VERSION"
  write_frontmatter "$dir/skills/foo/SKILL.md" "version: \"$version\""
  write_cargo "$dir" "$version"
  write_npm_package "$dir" "$version"
  write_plugin "$dir" "$version" "$version"
}

write_plugin() { # write_plugin <dir> <plugin-version-or-empty> <npx-pin-version-or-empty> -> an empty argument drops that declaration
  local version_line="" pin_arg='"--version"'
  [[ -n "$2" ]] && version_line="  \"version\": \"$2\","
  [[ -n "$3" ]] && pin_arg="\"@ejklock/living-docs@$3\""
  mkdir -p "$1/.claude-plugin"
  {
    echo '{'
    echo '  "name": "fixture",'
    [[ -n "$version_line" ]] && echo "$version_line"
    echo '  "mcpServers": {'
    echo '    "living-docs": {'
    echo '      "command": "npx",'
    echo "      \"args\": [\"-y\", $pin_arg, \"mcp\"]"
    echo '    }'
    echo '  }'
    echo '}'
  } >"$1/.claude-plugin/plugin.json"
}

write_npm_package() { # write_npm_package <dir> <version>
  mkdir -p "$1/npm"
  printf '{\n  "name": "fixture",\n  "version": "%s"\n}\n' "$2" >"$1/npm/package.json"
}

write_spec_skill() { # write_spec_skill <dir> <skill-md-version> -> a skill dir vendoring reference/SPEC.md at 0.1
  local dir="$1" version="$2"
  write_frontmatter "$dir/skills/spec-skill/SKILL.md" "version: \"$version\""
  mkdir -p "$dir/skills/spec-skill/reference"
  {
    echo "# Fixture Spec"
    echo
    echo "**Version 0.1 — Draft**"
  } >"$dir/skills/spec-skill/reference/SPEC.md"
}

new_repo() { # new_repo <name> -> prints the path to a consistent baseline repo at 0.9.0
  local dir="$TMP/$1"
  mkdir -p "$dir/scripts" "$dir/skills/foo"
  cp "$SCRIPT_SRC" "$dir/scripts/check-version.sh"
  chmod +x "$dir/scripts/check-version.sh"
  set_all_versions "$dir" "0.9.0"
  printf '%s' "$dir"
}

invoke() { # invoke <repo-dir> [args...]
  local dir="$1"
  shift
  OUT="$("$dir/scripts/check-version.sh" "$@" 2>&1)"
  RC=$?
}

check() { # check <name> <ok:0|1>
  if [[ "$2" == 1 ]]; then
    printf '  ok    %s\n' "$1"
  else
    printf '  FAIL  %s\n' "$1"
    printf '        exit=%s\n%s\n' "$RC" "$OUT" | sed 's/^/        out | /'
    fail=1
  fi
}

assert_exit() { # assert_exit <name> <expected>
  local ok=0
  [[ "$RC" == "$2" ]] && ok=1
  check "$1" "$ok"
}

assert_out_has() { # assert_out_has <name> <substring>
  local ok=0
  grep -qF -- "$2" <<<"$OUT" && ok=1
  check "$1" "$ok"
}

echo "check-version fixtures"
echo

echo "case 1: fully consistent tree"
repo="$(new_repo case1)"
invoke "$repo"
assert_exit    "1-exit-0"      0
assert_out_has "1-version-ok"  "Version OK"

echo "case 2: a skills/*/SKILL.md drifted"
repo="$(new_repo case2)"
write_frontmatter "$repo/skills/foo/SKILL.md" 'version: "0.0.1"'
invoke "$repo"
assert_exit    "2-exit-1"     1
assert_out_has "2-names-file" "skills/foo/SKILL.md"

echo "case 6: regression — a skills/*/SKILL.md with a space before the colon"
repo="$(new_repo case6)"
write_frontmatter "$repo/skills/foo/SKILL.md" 'version : "0.0.1"'
invoke "$repo"
assert_exit    "6-exit-1"        1
assert_out_has "6-malformed"     "MALFORMED"
assert_out_has "6-names-file"    "skills/foo/SKILL.md"

echo "case 9: skills/*/SKILL.md with no version key at all — required, so it fails"
repo="$(new_repo case9)"
write_frontmatter "$repo/skills/foo/SKILL.md" ""
invoke "$repo"
assert_exit "9-exit-1" 1

echo "case 10: explicit tag argument, matching tree"
repo="$(new_repo case10a)"
set_all_versions "$repo" "1.2.3"
invoke "$repo" v1.2.3
assert_exit    "10a-exit-0"     0
assert_out_has "10a-version-ok" "Version OK: 1.2.3"

echo "case 10: explicit tag argument, mismatched tree"
repo="$(new_repo case10b)"
invoke "$repo" v1.2.3
assert_exit "10b-exit-1" 1

echo "case 11: empty glob for a required class"
repo="$(new_repo case11)"
rm -rf "${repo:?}/skills"
invoke "$repo"
assert_exit    "11-exit-1"    1
assert_out_has "11-error"     "ERROR: no skills/*/SKILL.md files found"

echo "case 12: cli/Cargo.toml [package] version drifted"
repo="$(new_repo case12)"
write_cargo "$repo" "0.0.1"
invoke "$repo"
assert_exit    "12-exit-1"     1
assert_out_has "12-names-file" "cli/Cargo.toml"

echo "case 13: cli/Cargo.toml [dependencies] version differs from the correct [package] version"
repo="$(new_repo case13)"
write_cargo_dep_before_package "$repo" "0.9.0" "0.0.1"
invoke "$repo"
assert_exit    "13-exit-0"     0
assert_out_has "13-version-ok" "Version OK"

echo "case 14: vendored-spec skill at its spec version, rest at repo version"
repo="$(new_repo case14)"
write_spec_skill "$repo" "0.1"
invoke "$repo"
assert_exit    "14-exit-0"     0
assert_out_has "14-version-ok" "Version OK"

echo "case 15: vendored-spec skill drifted from its spec version"
repo="$(new_repo case15)"
write_spec_skill "$repo" "0.0.9"
invoke "$repo"
assert_exit    "15-exit-1"        1
assert_out_has "15-names-file"    "skills/spec-skill/SKILL.md"
assert_out_has "15-names-spec"    "reference/SPEC.md"

echo "case 16: vendored-spec skill pinned to the repo version instead of the spec version"
repo="$(new_repo case16)"
write_spec_skill "$repo" "0.9.0"
invoke "$repo"
assert_exit "16-exit-1" 1

echo "case 17: npm/package.json version drifted"
repo="$(new_repo case17)"
write_npm_package "$repo" "0.0.1"
invoke "$repo"
assert_exit    "17-exit-1"     1
assert_out_has "17-names-file" "npm/package.json"

echo "case 18: npm/package.json version matches the release version"
repo="$(new_repo case18)"
write_npm_package "$repo" "0.9.0"
invoke "$repo"
assert_exit    "18-exit-0"     0
assert_out_has "18-version-ok" "Version OK"

echo "case 19: mcp/Cargo.toml [package] version drifted"
repo="$(new_repo case19)"
write_cargo "$repo" "0.9.0" "0.0.1"
invoke "$repo"
assert_exit    "19-exit-1"     1
assert_out_has "19-names-file" "MISMATCH: mcp/Cargo.toml"

echo "case 20: mcp/Cargo.toml missing"
repo="$(new_repo case20)"
rm "$repo/mcp/Cargo.toml"
invoke "$repo"
assert_exit    "20-exit-1"     1
assert_out_has "20-names-file" "mcp/Cargo.toml"

echo "case 21: plugin version drifted"
repo="$(new_repo case21)"
write_plugin "$repo" "0.0.1" "0.9.0"
invoke "$repo"
assert_exit    "21-exit-1"     1
assert_out_has "21-names-file" "MISMATCH: .claude-plugin/plugin.json"

echo "case 22: plugin npx pin drifted"
repo="$(new_repo case22)"
write_plugin "$repo" "0.9.0" "0.0.1"
invoke "$repo"
assert_exit    "22-exit-1"    1
assert_out_has "22-names-pin" "@ejklock/living-docs@ pin"

echo "case 23: plugin with no version key"
repo="$(new_repo case23)"
write_plugin "$repo" "" "0.9.0"
invoke "$repo"
assert_exit    "23-exit-1"     1
assert_out_has "23-names-file" ".claude-plugin/plugin.json"

echo "case 24: plugin args hold no npx pin"
repo="$(new_repo case24)"
write_plugin "$repo" "0.9.0" ""
invoke "$repo"
assert_exit    "24-exit-1"    1
assert_out_has "24-names-pin" "@ejklock/living-docs@ pin"

echo "case 25: plugin manifest missing"
repo="$(new_repo case25)"
rm "$repo/.claude-plugin/plugin.json"
invoke "$repo"
assert_exit    "25-exit-1"     1
assert_out_has "25-names-file" ".claude-plugin/plugin.json"

echo "case 26: plugin version and pin both agree"
repo="$(new_repo case26)"
write_plugin "$repo" "0.9.0" "0.9.0"
invoke "$repo"
assert_exit    "26-exit-0"     0
assert_out_has "26-version-ok" "Version OK"

echo "case 27: tag argument ahead of a plugin still at the baseline"
repo="$(new_repo case27)"
write_cargo "$repo" "0.9.1"
write_npm_package "$repo" "0.9.1"
write_frontmatter "$repo/skills/foo/SKILL.md" 'version: "0.9.1"'
printf '%s' "0.9.1" >"$repo/VERSION"
invoke "$repo" v0.9.1
assert_exit    "27-exit-1"     1
assert_out_has "27-names-file" "MISMATCH: .claude-plugin/plugin.json"

write_plugin_two_pins() { # write_plugin_two_pins <dir> <first-pin-version> <second-pin-version>
  mkdir -p "$1/.claude-plugin"
  cat >"$1/.claude-plugin/plugin.json" <<JSON
{
  "name": "fixture",
  "version": "0.9.0",
  "mcpServers": {
    "living-docs": {
      "command": "npx",
      "args": ["-y", "@ejklock/living-docs@$2", "mcp"]
    },
    "living-docs-again": {
      "command": "npx",
      "args": ["-y", "@ejklock/living-docs@$3", "mcp"]
    }
  }
}
JSON
}

write_plugin_nested_version() { # write_plugin_nested_version <dir> <nested-version> <top-level-version>
  mkdir -p "$1/.claude-plugin"
  cat >"$1/.claude-plugin/plugin.json" <<JSON
{
  "name": "fixture",
  "author": {
    "name": "someone",
    "version": "$2"
  },
  "version": "$3",
  "mcpServers": {
    "living-docs": {
      "command": "npx",
      "args": ["-y", "@ejklock/living-docs@0.9.0", "mcp"]
    }
  }
}
JSON
}

echo "case 28: plugin with a second npx pin at another version"
repo="$(new_repo case28)"
write_plugin_two_pins "$repo" "0.9.0" "0.0.1"
invoke "$repo"
assert_exit    "28-exit-1"    1
assert_out_has "28-names-pin" "@ejklock/living-docs@ pin"
assert_out_has "28-names-got" "'0.0.1'"

echo "case 29: plugin with two npx pins that both agree"
repo="$(new_repo case29)"
write_plugin_two_pins "$repo" "0.9.0" "0.9.0"
invoke "$repo"
assert_exit    "29-exit-0"     0
assert_out_has "29-version-ok" "Version OK"

echo "case 30: nested version equal to the release before a wrong top-level version"
repo="$(new_repo case30)"
write_plugin_nested_version "$repo" "0.9.0" "0.0.1"
invoke "$repo"
assert_exit    "30-exit-1"     1
assert_out_has "30-names-file" "MISMATCH: .claude-plugin/plugin.json"

echo "case 31: nested version that differs while the top-level version agrees"
repo="$(new_repo case31)"
write_plugin_nested_version "$repo" "0.0.1" "0.9.0"
invoke "$repo"
assert_exit    "31-exit-0"     0
assert_out_has "31-version-ok" "Version OK"

echo
if ((fail == 0)); then
  echo "All check-version fixtures passed."
  exit 0
else
  echo "check-version fixture failures."
  exit 1
fi
