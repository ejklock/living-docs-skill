#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/check-file-size.sh.
#
# check-file-size.sh derives its `root` from its own location (dirname "$0"/..), so each case
# builds a synthetic repo under mktemp -d and runs a COPY of the real script from
# $TMP/<case>/scripts/check-file-size.sh against that synthetic tree. No network access.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
SCRIPT_SRC="$HERE/../../check-file-size.sh"
TMP="$(mktemp -d)"
trap 'chmod -R u+rwx "$TMP" 2>/dev/null; rm -rf "$TMP"' EXIT

fail=0

write_rs_file() { # write_rs_file <path> <lines>
  local file="$1" n="$2"
  mkdir -p "$(dirname "$file")"
  yes 'x' | head -n "$n" >"$file"
}

write_baseline() { # write_baseline <dir> <entry>...
  local dir="$1" entry
  shift
  : >"$dir/scripts/file-size-baseline.txt"
  for entry in "$@"; do
    echo "$entry" >>"$dir/scripts/file-size-baseline.txt"
  done
}

new_repo() { # new_repo <name> -> prints the path to an empty synthetic repo
  local dir="$TMP/$1"
  mkdir -p "$dir/scripts"
  cp "$SCRIPT_SRC" "$dir/scripts/check-file-size.sh"
  chmod +x "$dir/scripts/check-file-size.sh"
  : >"$dir/scripts/file-size-baseline.txt"
  printf '%s' "$dir"
}

invoke() { # invoke <repo-dir>
  local dir="$1"
  OUT="$("$dir/scripts/check-file-size.sh" 2>&1)"
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

echo "check-file-size fixtures"
echo

echo "case 1: clean tree, all files under 300 lines"
repo="$(new_repo case1)"
write_rs_file "$repo/src/lib.rs" 299
invoke "$repo"
assert_exit    "1-exit-0" 0
assert_out_has "1-ok"     "File-size ratchet OK"

echo "case 2: non-grandfathered file over 300 lines"
repo="$(new_repo case2)"
write_rs_file "$repo/src/lib.rs" 301
invoke "$repo"
assert_exit    "2-exit-1"     1
assert_out_has "2-over-limit" "OVER LIMIT: src/lib.rs"

echo "case 3: grandfathered file exactly at its baseline count"
repo="$(new_repo case3)"
write_rs_file "$repo/src/big.rs" 400
write_baseline "$repo" "400 src/big.rs"
invoke "$repo"
assert_exit    "3-exit-0" 0
assert_out_has "3-ok"     "File-size ratchet OK"

echo "case 4: grandfathered file one line over its baseline"
repo="$(new_repo case4)"
write_rs_file "$repo/src/big.rs" 401
write_baseline "$repo" "400 src/big.rs"
invoke "$repo"
assert_exit    "4-exit-1" 1
assert_out_has "4-grew"   "GREW: src/big.rs has 401 lines, exceeds its baseline of 400"

echo "case 5: grandfathered file now at or under 300 lines but still listed"
repo="$(new_repo case5)"
write_rs_file "$repo/src/big.rs" 300
write_baseline "$repo" "400 src/big.rs"
invoke "$repo"
assert_exit    "5-exit-1" 1
assert_out_has "5-stale"  "remove the entry — the file leaves the list permanently"

echo "case 6: baseline entry for a missing path"
repo="$(new_repo case6)"
write_baseline "$repo" "400 src/gone.rs"
invoke "$repo"
assert_exit    "6-exit-1"        1
assert_out_has "6-stale-missing" "STALE BASELINE: src/gone.rs no longer exists"

echo "case 7: grandfathered file shrunk but still over 300 lines"
repo="$(new_repo case7)"
write_rs_file "$repo/src/big.rs" 350
write_baseline "$repo" "400 src/big.rs"
invoke "$repo"
assert_exit    "7-exit-0"   0
assert_out_has "7-advisory" "ADVISORY: src/big.rs shrank to 350 lines (baseline 400)"

echo "case 8: repo located under .claude/worktrees/ gives the verdict of any other location"
repo="$(new_repo .claude/worktrees/wt8)"
write_rs_file "$repo/src/big.rs" 301
write_baseline "$repo" "301 src/big.rs"
invoke "$repo"
assert_exit    "8-exit-0" 0
assert_out_has "8-ok"     "File-size ratchet OK"
write_rs_file "$repo/src/big.rs" 302
invoke "$repo"
assert_exit    "8-grew-exit-1" 1
assert_out_has "8-grew"        "GREW: src/big.rs has 302 lines, exceeds its baseline of 301"

echo "case 9: .rs file inside a nested worktree of the repo is not counted"
repo="$(new_repo case9)"
write_rs_file "$repo/src/lib.rs" 10
write_rs_file "$repo/.claude/worktrees/other/src/huge.rs" 301
invoke "$repo"
assert_exit    "9-exit-0" 0
assert_out_has "9-ok"     "File-size ratchet OK"

echo "case 10: repo whose path holds a glob bracket still ignores its nested worktrees"
repo="$(new_repo 'a[1]/repo')"
write_rs_file "$repo/src/lib.rs" 10
write_rs_file "$repo/.claude/worktrees/other/src/huge.rs" 301
invoke "$repo"
assert_exit    "10-exit-0" 0
assert_out_has "10-ok"     "File-size ratchet OK"

echo "case 11: repo under a target/ directory counts its own files"
repo="$(new_repo target/repo)"
write_rs_file "$repo/src/big.rs" 301
write_baseline "$repo" "301 src/big.rs"
invoke "$repo"
assert_exit    "11-exit-0" 0
assert_out_has "11-ok"     "File-size ratchet OK"

echo "case 12: root-level target/ and .git/ are ignored, a nested src/target/ is counted"
repo="$(new_repo case12)"
write_rs_file "$repo/target/debug/huge.rs" 301
write_rs_file "$repo/.git/huge.rs" 301
invoke "$repo"
assert_exit    "12-exit-0" 0
write_rs_file "$repo/src/target/x.rs" 301
invoke "$repo"
assert_exit    "12-nested-exit-1" 1
assert_out_has "12-nested"        "OVER LIMIT: src/target/x.rs"

if [[ "$(id -u)" == 0 ]]; then
  echo "skip: unreadable-directory cases (root reads a mode-000 directory)"
else
  echo "case 13: unreadable directory holding an over-limit file fails with the enumeration error"
  repo="$(new_repo case13)"
  write_rs_file "$repo/src/locked/y.rs" 301
  chmod 000 "$repo/src/locked"
  invoke "$repo"
  chmod 755 "$repo/src/locked"
  assert_exit      "13-exit-1"  1
  assert_out_has   "13-named"   "cannot enumerate .rs files"
  assert_out_lacks "13-no-ok"   "ratchet OK"

  echo "case 14: unreadable directory holding only a small file still fails"
  repo="$(new_repo case14)"
  write_rs_file "$repo/src/lib.rs" 10
  write_rs_file "$repo/src/locked/small.rs" 5
  chmod 000 "$repo/src/locked"
  invoke "$repo"
  chmod 755 "$repo/src/locked"
  assert_exit      "14-exit-1" 1
  assert_out_has   "14-named"  "cannot enumerate .rs files"
  assert_out_lacks "14-no-ok"  "ratchet OK"

  echo "case 15: unreadable directory under the pruned target/ is not read"
  repo="$(new_repo case15)"
  write_rs_file "$repo/src/lib.rs" 10
  write_rs_file "$repo/target/locked/x.rs" 5
  chmod 000 "$repo/target/locked"
  invoke "$repo"
  chmod 755 "$repo/target/locked"
  assert_exit    "15-exit-0" 0
  assert_out_has "15-ok"     "File-size ratchet OK"
fi

echo
if ((fail == 0)); then
  echo "All check-file-size fixtures passed."
  exit 0
else
  echo "check-file-size fixture failures."
  exit 1
fi
