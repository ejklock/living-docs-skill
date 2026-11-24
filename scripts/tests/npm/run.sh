#!/usr/bin/env bash
#
# run.sh — fixture tests for the npm package under npm/.
#
# The packed tarball is installed into a throwaway project while
# LIVING_DOCS_DOWNLOAD_BASE points at a one-file Node HTTP stub on an ephemeral
# 127.0.0.1 port; the stub serves a fake executable asset plus its .sha256 from a temp
# directory and logs every request. The stub is the only network: npm runs offline with a
# private cache, and nothing reaches the registry or GitHub.
#
# Usage:  run.sh [case ...]   (default: every case)
# Exit:   0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../../.." && pwd)"
PKG_DIR="$REPO_ROOT/npm"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/living-docs-npm-tests.XXXXXX")"
STUB_PID=""
cleanup() {
  [[ -n "$STUB_PID" ]] && kill "$STUB_PID" 2>/dev/null
  rm -rf "$TMP"
}
trap cleanup EXIT

VERSION="$(tr -d '[:space:]' <"$REPO_ROOT/VERSION")"
ASSET="living-docs-x86_64-unknown-linux-gnu"
FAKE_OUTPUT="living-docs fake-asset 1.2.3"
SERVE_DIR="$TMP/serve"
ACCESS_LOG="$TMP/access.log"
PORT_FILE="$TMP/port"
TARBALL=""
fail=0

export npm_config_cache="$TMP/npm-cache"
export npm_config_audit=false npm_config_fund=false npm_config_update_notifier=false
export npm_config_offline=true
export LIVING_DOCS_PLATFORM="linux-x64"
export HOME="$TMP/home"
export XDG_CACHE_HOME="$TMP/xdg-cache"
mkdir -p "$HOME"

write_stub_server() {
  cat >"$TMP/stub.js" <<'JS'
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const [root, logFile, portFile] = process.argv.slice(2);
http.createServer((req, res) => {
  fs.appendFileSync(logFile, `${req.method} ${req.url}\n`);
  const file = path.join(root, path.normalize(req.url));
  if (!file.startsWith(root) || !fs.existsSync(file)) {
    res.writeHead(404).end();
    return;
  }
  res.writeHead(200).end(fs.readFileSync(file));
}).listen(0, '127.0.0.1', function () {
  fs.writeFileSync(portFile, String(this.address().port));
});
JS
}

start_stub() {
  : >"$ACCESS_LOG"
  write_stub_server
  node "$TMP/stub.js" "$SERVE_DIR" "$ACCESS_LOG" "$PORT_FILE" &
  STUB_PID=$!
  for _ in $(seq 1 50); do
    [[ -s "$PORT_FILE" ]] && return 0
    sleep 0.1
  done
  echo "stub server did not start" >&2
  return 1
}

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

publish_asset() { # publish_asset <good|bad> -> writes the fake asset and its checksum under v<version>/
  local dir="$SERVE_DIR/v$VERSION"
  mkdir -p "$dir"
  printf '#!/bin/sh\necho "%s"\n' "$FAKE_OUTPUT" >"$dir/$ASSET"
  if [[ "$1" == good ]]; then
    printf '%s  %s\n' "$(sha256_of "$dir/$ASSET")" "$ASSET" >"$dir/$ASSET.sha256"
  else
    printf '%064d  %s\n' 0 "$ASSET" >"$dir/$ASSET.sha256"
  fi
}

pack_tarball() {
  local out
  out="$(cd "$PKG_DIR" && npm pack --pack-destination "$TMP" --silent 2>&1)" || {
    printf 'npm pack failed:\n%s\n' "$out" >&2
    return 1
  }
  TARBALL="$TMP/$(tail -n1 <<<"$out")"
}

new_project() { # new_project <name> -> prints the project dir
  local dir="$TMP/$1"
  mkdir -p "$dir"
  printf '{"name":"fixture","version":"1.0.0","private":true}\n' >"$dir/package.json"
  printf '%s' "$dir"
}

begin_case() { # begin_case <good|bad>: fresh served dir, stub, tarball
  rm -rf "$SERVE_DIR" "$PORT_FILE"
  publish_asset "$1"
  start_stub || return 1
  pack_tarball || return 1
  local port
  port="$(cat "$PORT_FILE")"
  export LIVING_DOCS_DOWNLOAD_BASE="http://127.0.0.1:$port/"
}

end_case() {
  [[ -n "$STUB_PID" ]] && kill "$STUB_PID" 2>/dev/null
  wait "$STUB_PID" 2>/dev/null
  STUB_PID=""
}

check() { # check <name> <ok:0|1> [detail]
  if [[ "$2" == 1 ]]; then
    printf '  ok    %s\n' "$1"
  else
    printf '  FAIL  %s\n' "$1"
    [[ -n "${3:-}" ]] && printf '%s\n' "$3" | sed 's/^/        | /'
    fail=1
  fi
}

installed_pkg() { printf '%s/node_modules/@ejklock/living-docs' "$1"; }

asset_fetch_count() { grep -c "GET /v$VERSION/$ASSET\$" "$ACCESS_LOG"; }

first_run_in() { # first_run_in <project> [env assignments...] -> stdout of the first shim run; stderr in $TMP/stderr
  local proj="$1"
  shift
  (cd "$proj" && env "$@" npx --no-install living-docs --version 2>"$TMP/stderr")
}

install_in() { # install_in <project> -> installs the tarball, prints the output
  (cd "$1" && npm install "$TARBALL" 2>&1)
}

case_no_install_script() {
  begin_case good || return 1
  local listing scripts
  listing="$(tar -tzf "$TARBALL")"
  scripts="$(tar -xzOf "$TARBALL" package/package.json | node -e '
    const pkg = JSON.parse(require("node:fs").readFileSync(0, "utf8"));
    const s = pkg.scripts || {};
    process.stdout.write(["preinstall", "install", "postinstall"].filter((k) => k in s).join(","));
  ')"
  check "packed-manifest-has-no-install-script" "$([[ -z "$scripts" ]] && echo 1 || echo 0)" "$scripts"
  check "tarball-lists-no-scripts-dir" "$(grep -q '^package/scripts/' <<<"$listing" && echo 0 || echo 1)" "$listing"
  end_case
}

case_first_run() {
  begin_case good || return 1
  local proj out rc first second cache="$TMP/first-run-cache"
  proj="$(new_project first)"
  out="$(install_in "$proj")"
  rc=$?
  check "install-exit-0" "$([[ $rc == 0 ]] && echo 1 || echo 0)" "$out"
  check "no-binary-after-install" "$([[ ! -e "$(installed_pkg "$proj")/vendor/living-docs" ]] && echo 1 || echo 0)"
  check "no-request-during-install" "$([[ ! -s "$ACCESS_LOG" ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  first="$(first_run_in "$proj" "XDG_CACHE_HOME=$cache")"
  check "first-run-stdout-is-only-asset-output" "$([[ "$first" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$first"
  check "one-asset-fetch-after-first-run" "$([[ "$(asset_fetch_count)" == 1 ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  second="$(first_run_in "$proj" "XDG_CACHE_HOME=$cache")"
  check "second-run-prints-asset-output" "$([[ "$second" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$second"
  check "no-fetch-on-second-run" "$([[ "$(asset_fetch_count)" == 1 ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  check "writable-package-uses-vendor" "$([[ -x "$(installed_pkg "$proj")/vendor/living-docs" ]] && echo 1 || echo 0)"
  check "writable-package-leaves-cache-untouched" "$([[ ! -e "$cache" ]] && echo 1 || echo 0)" "$(ls -R "$cache" 2>&1)"
  end_case
}

case_bad_checksum() {
  begin_case bad || return 1
  local proj out rc first_rc leftover cache="$TMP/bad-cache" stderr
  proj="$(new_project bad)"
  out="$(install_in "$proj")"
  rc=$?
  check "install-exit-0" "$([[ $rc == 0 ]] && echo 1 || echo 0)" "$out"
  first_run_in "$proj" "XDG_CACHE_HOME=$cache" >/dev/null
  first_rc=$?
  stderr="$(cat "$TMP/stderr")"
  check "first-run-exit-nonzero" "$([[ $first_rc != 0 ]] && echo 1 || echo 0)" "$stderr"
  leftover="$(find "$(installed_pkg "$proj")/vendor" "$cache" -type f 2>/dev/null)"
  check "no-binary-or-partial-file" "$([[ -z "$leftover" ]] && echo 1 || echo 0)" "$leftover"
  check "names-install-sh-fallback" "$(grep -q 'install.sh' <<<"$stderr" && echo 1 || echo 0)" "$stderr"
  end_case
}

case_readonly_package() {
  begin_case good || return 1
  local proj pkg cache="$TMP/ro-cache" home="$TMP/ro-home" first second out rc version
  local other_version="0.0.1-other"
  proj="$(new_project readonly)"
  out="$(install_in "$proj")"
  pkg="$(installed_pkg "$proj")"
  version="$(node -p "require('$pkg/package.json').version")"
  mkdir -p "$cache/living-docs/$other_version"
  printf '#!/bin/sh\necho "other version binary"\n' >"$cache/living-docs/$other_version/living-docs"
  chmod 755 "$cache/living-docs/$other_version/living-docs"
  chmod a-w "$pkg"
  first="$(first_run_in "$proj" "XDG_CACHE_HOME=$cache")"
  chmod u+w "$pkg"
  check "first-run-prints-asset-output" "$([[ "$first" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$first$(cat "$TMP/stderr")"
  check "binary-written-to-versioned-cache" "$([[ -x "$cache/living-docs/$version/living-docs" ]] && echo 1 || echo 0)" "$(ls -R "$cache" 2>&1)"
  check "no-binary-in-package" "$([[ ! -e "$pkg/vendor/living-docs" ]] && echo 1 || echo 0)"
  check "other-version-cache-never-run" "$([[ "$first" != *"other version"* ]] && echo 1 || echo 0)" "$first"
  chmod a-w "$pkg"
  second="$(first_run_in "$proj" "XDG_CACHE_HOME=$cache")"
  chmod u+w "$pkg"
  check "second-run-reuses-cache" "$([[ "$second" == "$FAKE_OUTPUT" && "$(asset_fetch_count)" == 1 ]] && echo 1 || echo 0)" "$second $(cat "$ACCESS_LOG")"

  rm -rf "$cache"
  mkdir -p "$home"
  chmod a-w "$pkg"
  first="$(first_run_in "$proj" "-u" "XDG_CACHE_HOME" "HOME=$home")"
  chmod u+w "$pkg"
  check "unset-xdg-falls-back-to-home-cache" "$([[ "$first" == "$FAKE_OUTPUT" && -x "$home/.cache/living-docs/$version/living-docs" ]] && echo 1 || echo 0)" "$first$(cat "$TMP/stderr")"

  chmod a-w "$pkg"
  (cd "$proj" && env -u XDG_CACHE_HOME -u HOME node "$pkg/bin/living-docs.js" --version >/dev/null 2>"$TMP/stderr")
  rc=$?
  chmod u+w "$pkg"
  check "no-home-fails-with-install-sh-hint" "$([[ $rc != 0 ]] && grep -q 'install.sh' "$TMP/stderr" && echo 1 || echo 0)" "$(cat "$TMP/stderr")"
  end_case
}

write_rename_failure_preload() { # a preload that makes renaming a partial file fail, as a full disk or a locked target would
  cat >"$TMP/fail-rename.js" <<'JS'
const fs = require('node:fs');
const realRename = fs.renameSync;
fs.renameSync = function (from, to) {
  if (/\.living-docs\..*\.partial$/.test(String(from))) {
    const error = new Error(`EXDEV: simulated rename failure, rename '${from}' -> '${to}'`);
    error.code = 'EXDEV';
    if (process.env.RENAME_FAILURE_LOG) fs.appendFileSync(process.env.RENAME_FAILURE_LOG, `${from}\n`);
    throw error;
  }
  return realRename.apply(this, arguments);
};
JS
}

case_write_failure() {
  begin_case good || return 1
  local proj pkg out first_rc stderr leftover cache="$TMP/wf-cache"
  proj="$(new_project writefail)"
  out="$(install_in "$proj")"
  pkg="$(installed_pkg "$proj")"
  write_rename_failure_preload

  (cd "$proj" && env "XDG_CACHE_HOME=$cache" "NODE_OPTIONS=--require=$TMP/fail-rename.js" node "$pkg/bin/living-docs.js" --version >/dev/null 2>"$TMP/stderr")
  first_rc=$?
  stderr="$(cat "$TMP/stderr")"
  leftover="$(find "$pkg/vendor" "$cache" -name '*.partial' 2>/dev/null)"
  check "vendor-rename-failure-exits-nonzero" "$([[ $first_rc != 0 ]] && echo 1 || echo 0)" "$stderr"
  check "vendor-rename-failure-names-install-sh" "$(grep -q 'install.sh' <<<"$stderr" && echo 1 || echo 0)" "$stderr"
  check "vendor-rename-failure-leaves-no-partial" "$([[ -z "$leftover" ]] && echo 1 || echo 0)" "$leftover"
  check "vendor-rename-failure-leaves-no-binary" "$([[ ! -e "$pkg/vendor/living-docs" ]] && echo 1 || echo 0)"

  local intercepted="$TMP/wf-intercepted"
  mkdir -p "$pkg/vendor"
  chmod a-w "$pkg/vendor"
  : >"$intercepted"
  (cd "$proj" && env "XDG_CACHE_HOME=$cache" "RENAME_FAILURE_LOG=$intercepted" "NODE_OPTIONS=--require=$TMP/fail-rename.js" node "$pkg/bin/living-docs.js" --version >/dev/null 2>"$TMP/stderr")
  first_rc=$?
  chmod u+w "$pkg/vendor"
  stderr="$(cat "$TMP/stderr")"
  leftover="$(find "$pkg/vendor" "$cache" -name '*.partial' 2>/dev/null)"
  check "cache-rename-failure-exits-nonzero" "$([[ $first_rc != 0 ]] && echo 1 || echo 0)" "$stderr"
  check "cache-rename-failure-names-install-sh" "$(grep -q 'install.sh' <<<"$stderr" && echo 1 || echo 0)" "$stderr"
  check "cache-rename-failure-leaves-no-partial" "$([[ -z "$leftover" ]] && echo 1 || echo 0)" "$leftover"
  check "cache-rename-failure-hit-a-partial-under-the-cache" "$([[ -s "$intercepted" ]] && ! grep -qv "^$cache/" "$intercepted" && echo 1 || echo 0)" "$(cat "$intercepted")"
  check "cache-rename-failure-reached-the-write" "$([[ "$(asset_fetch_count)" -ge 1 ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  end_case
}

case_readonly_vendor() {
  begin_case good || return 1
  local proj pkg out first cache="$TMP/rv-cache" version
  proj="$(new_project readonlyvendor)"
  out="$(install_in "$proj")"
  pkg="$(installed_pkg "$proj")"
  version="$(node -p "require('$pkg/package.json').version")"
  mkdir -p "$pkg/vendor"
  chmod a-w "$pkg/vendor"
  first="$(first_run_in "$proj" "XDG_CACHE_HOME=$cache")"
  chmod u+w "$pkg/vendor"
  check "first-run-prints-asset-output" "$([[ "$first" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$first$(cat "$TMP/stderr")"
  check "binary-written-to-versioned-cache" "$([[ -x "$cache/living-docs/$version/living-docs" ]] && echo 1 || echo 0)" "$(ls -R "$cache" 2>&1)"
  check "nothing-written-to-readonly-vendor" "$([[ -z "$(ls -A "$pkg/vendor")" ]] && echo 1 || echo 0)" "$(ls -A "$pkg/vendor")"
  end_case
}

case_no_location() {
  begin_case good || return 1
  local proj pkg out rc stderr
  proj="$(new_project nolocation)"
  out="$(install_in "$proj")"
  pkg="$(installed_pkg "$proj")"
  chmod a-w "$pkg"
  (cd "$proj" && env -u XDG_CACHE_HOME -u HOME node "$pkg/bin/living-docs.js" --version >/dev/null 2>"$TMP/stderr")
  rc=$?
  chmod u+w "$pkg"
  stderr="$(cat "$TMP/stderr")"
  check "exit-nonzero" "$([[ $rc != 0 ]] && echo 1 || echo 0)" "$stderr"
  check "names-install-sh-fallback" "$(grep -q 'install.sh' <<<"$stderr" && echo 1 || echo 0)" "$stderr"
  check "no-asset-request-before-target-is-known" "$([[ "$(asset_fetch_count)" == 0 ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  end_case
}

case_relative_xdg() {
  begin_case good || return 1
  local proj pkg out first home="$TMP/rx-home" version
  proj="$(new_project relativexdg)"
  out="$(install_in "$proj")"
  pkg="$(installed_pkg "$proj")"
  version="$(node -p "require('$pkg/package.json').version")"
  mkdir -p "$home"
  chmod a-w "$pkg"
  first="$(first_run_in "$proj" "XDG_CACHE_HOME=relative/cache" "HOME=$home")"
  chmod u+w "$pkg"
  check "first-run-prints-asset-output" "$([[ "$first" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$first$(cat "$TMP/stderr")"
  check "binary-written-under-home-cache" "$([[ -x "$home/.cache/living-docs/$version/living-docs" ]] && echo 1 || echo 0)" "$(ls -R "$home" 2>&1)"
  check "nothing-created-under-relative-path" "$([[ ! -e "$proj/relative" ]] && echo 1 || echo 0)" "$(ls -R "$proj/relative" 2>&1)"
  end_case
}

case_unsupported_platform() {
  begin_case good || return 1
  local proj out rc target missing=0 stderr
  proj="$(new_project unsupported)"
  out="$(cd "$proj" && LIVING_DOCS_PLATFORM="win32-x64" npm install "$TARBALL" 2>&1)"
  rc=$?
  check "install-exit-0" "$([[ $rc == 0 ]] && echo 1 || echo 0)" "$out"
  first_run_in "$proj" "LIVING_DOCS_PLATFORM=win32-x64" >/dev/null
  rc=$?
  stderr="$(cat "$TMP/stderr")"
  check "first-run-exit-nonzero" "$([[ $rc != 0 ]] && echo 1 || echo 0)" "$stderr"
  for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    grep -qF "$target" <<<"$stderr" || missing=1
  done
  check "lists-four-supported-targets" "$([[ $missing == 0 ]] && echo 1 || echo 0)" "$stderr"
  end_case
}

CASES=(no_install_script first_run bad_checksum readonly_package write_failure readonly_vendor no_location relative_xdg unsupported_platform)
selected=("$@")
[[ ${#selected[@]} -eq 0 ]] && selected=("${CASES[@]}")

echo "npm package fixtures"
for name in "${selected[@]}"; do
  if ! declare -F "case_$name" >/dev/null; then
    echo "unknown case: $name (cases: ${CASES[*]})" >&2
    exit 1
  fi
  echo "case: $name"
  "case_$name" || { echo "  FAIL  setup"; fail=1; }
done

echo
if ((fail == 0)); then
  echo "All npm package fixtures passed."
  exit 0
else
  echo "npm package fixture failures."
  exit 1
fi
