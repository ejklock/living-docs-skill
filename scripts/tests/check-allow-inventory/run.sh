#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/check-allow-inventory.sh.
#
# check-allow-inventory.sh derives its `root` from its own location (dirname "$0"/..), so
# each case builds a synthetic repo under mktemp -d and runs a COPY of the real script from
# $TMP/<case>/scripts/check-allow-inventory.sh against that synthetic tree. No network access.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT_SRC="$HERE/../../check-allow-inventory.sh"
TMP="$(mktemp -d)"
trap 'chmod -R u+rwx "$TMP" 2>/dev/null; rm -rf "$TMP"' EXIT

fail=0
marker='#[allow(clippy::too_many_lines)]'

write_rs_file() { # write_rs_file <path> <allow-count>
  local file="$1" n="$2" i
  mkdir -p "$(dirname "$file")"
  : >"$file"
  for ((i = 0; i < n; i++)); do
    printf '%s\nfn f_%s() {}\n' "$marker" "$i" >>"$file"
  done
}

write_baseline() { # write_baseline <dir> <entry>...
  local dir="$1" entry
  shift
  : >"$dir/scripts/allow-inventory-baseline.txt"
  for entry in "$@"; do
    echo "$entry" >>"$dir/scripts/allow-inventory-baseline.txt"
  done
}

new_repo() { # new_repo <name> -> prints the path to an empty synthetic repo
  local dir="$TMP/$1"
  mkdir -p "$dir/scripts"
  cp "$SCRIPT_SRC" "$dir/scripts/check-allow-inventory.sh"
  chmod +x "$dir/scripts/check-allow-inventory.sh"
  : >"$dir/scripts/allow-inventory-baseline.txt"
  printf '%s' "$dir"
}

invoke() { # invoke <repo-dir>
  local dir="$1"
  OUT="$("$dir/scripts/check-allow-inventory.sh" 2>&1)"
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

assert_out_lacks() { # assert_out_lacks <name> <substring>
  local ok=1
  grep -qF -- "$2" <<<"$OUT" && ok=0
  check "$1" "$ok"
}

echo "check-allow-inventory fixtures"
echo

echo "case 1: clean tree, baseline matches reality"
repo="$(new_repo case1)"
write_rs_file "$repo/src/lib.rs" 3
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "1-exit-0" 0
assert_out_has "1-ok"     "Allow-inventory gate OK"

echo "case 2: listed file grew past its baseline"
repo="$(new_repo case2)"
write_rs_file "$repo/src/lib.rs" 4
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "2-exit-1" 1
assert_out_has "2-grew"   "GREW: src/lib.rs has 4 allow(s), exceeds its baseline of 3"

echo "case 3: non-listed file gained an allow"
repo="$(new_repo case3)"
write_rs_file "$repo/src/other.rs" 1
invoke "$repo"
assert_exit    "3-exit-1"    1
assert_out_has "3-new-allow" "NEW ALLOW: src/other.rs has 1 allow(s) but is not in the baseline"

echo "case 4: listed file dropped to zero allows but the entry stayed"
repo="$(new_repo case4)"
write_rs_file "$repo/src/lib.rs" 0
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "4-exit-1" 1
assert_out_has "4-stale"  "STALE BASELINE: src/lib.rs now has zero allows"

echo "case 5: baseline entry for a missing path"
repo="$(new_repo case5)"
write_baseline "$repo" "3 src/gone.rs"
invoke "$repo"
assert_exit    "5-exit-1"        1
assert_out_has "5-stale-missing" "STALE BASELINE: src/gone.rs no longer exists"

echo "case 6: listed file shrunk but still above zero"
repo="$(new_repo case6)"
write_rs_file "$repo/src/lib.rs" 2
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "6-exit-0"   0
assert_out_has "6-advisory" "ADVISORY: src/lib.rs shrank to 2 allow(s) (baseline 3)"

echo "case 7: repo located under .claude/worktrees/ gives the verdict of any other location"
repo="$(new_repo .claude/worktrees/wt7)"
write_rs_file "$repo/src/lib.rs" 3
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "7-exit-0" 0
assert_out_has "7-ok"     "Allow-inventory gate OK"
write_rs_file "$repo/src/lib.rs" 4
invoke "$repo"
assert_exit    "7-grew-exit-1" 1
assert_out_has "7-grew"        "GREW: src/lib.rs has 4 allow(s), exceeds its baseline of 3"

echo "case 8: allow inside a nested worktree of the repo is not counted"
repo="$(new_repo case8)"
write_rs_file "$repo/src/lib.rs" 0
write_rs_file "$repo/.claude/worktrees/other/src/x.rs" 2
invoke "$repo"
assert_exit    "8-exit-0" 0
assert_out_has "8-ok"     "Allow-inventory gate OK"

echo "case 9: repo whose path holds a glob bracket still ignores its nested worktrees"
repo="$(new_repo 'a[1]/repo')"
write_rs_file "$repo/src/lib.rs" 0
write_rs_file "$repo/.claude/worktrees/other/src/x.rs" 2
invoke "$repo"
assert_exit    "9-exit-0" 0
assert_out_has "9-ok"     "Allow-inventory gate OK"

echo "case 10: repo under a target/ directory counts its own files"
repo="$(new_repo target/repo)"
write_rs_file "$repo/src/lib.rs" 3
write_baseline "$repo" "3 src/lib.rs"
invoke "$repo"
assert_exit    "10-exit-0" 0
assert_out_has "10-ok"     "Allow-inventory gate OK"

echo "case 11: root-level target/ and .git/ are ignored, a nested src/target/ is counted"
repo="$(new_repo case11)"
write_rs_file "$repo/target/debug/x.rs" 2
write_rs_file "$repo/.git/x.rs" 2
invoke "$repo"
assert_exit    "11-exit-0" 0
write_rs_file "$repo/src/target/x.rs" 2
invoke "$repo"
assert_exit    "11-nested-exit-1" 1
assert_out_has "11-nested"        "NEW ALLOW: src/target/x.rs"

if [[ "$(id -u)" == 0 ]]; then
  echo "skip: unreadable-directory cases (root reads a mode-000 directory)"
else
  echo "case 12: unreadable directory holding a new allow fails with the enumeration error"
  repo="$(new_repo case12)"
  write_rs_file "$repo/src/locked/y.rs" 1
  chmod 000 "$repo/src/locked"
  invoke "$repo"
  chmod 755 "$repo/src/locked"
  assert_exit      "12-exit-1" 1
  assert_out_has   "12-named"  "cannot enumerate .rs files"
  assert_out_lacks "12-no-ok"  "gate OK"

  echo "case 13: unreadable directory holding a clean file still fails"
  repo="$(new_repo case13)"
  write_rs_file "$repo/src/lib.rs" 0
  write_rs_file "$repo/src/locked/clean.rs" 0
  chmod 000 "$repo/src/locked"
  invoke "$repo"
  chmod 755 "$repo/src/locked"
  assert_exit      "13-exit-1" 1
  assert_out_has   "13-named"  "cannot enumerate .rs files"
  assert_out_lacks "13-no-ok"  "gate OK"
fi

echo
if ((fail == 0)); then
  echo "All check-allow-inventory fixtures passed."
  exit 0
else
  echo "check-allow-inventory fixture failures."
  exit 1
fi
