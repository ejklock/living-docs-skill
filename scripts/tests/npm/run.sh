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

installed_pkg() { printf '%s/node_modules/living-docs' "$1"; }

asset_fetch_count() { grep -c "GET /v$VERSION/$ASSET\$" "$ACCESS_LOG"; }

case_good_checksum() {
  begin_case good || return 1
  local proj out rc ok=0 version_out
  proj="$(new_project good)"
  out="$(cd "$proj" && npm install "$TARBALL" 2>&1)"
  rc=$?
  check "install-exit-0" "$([[ $rc == 0 ]] && echo 1 || echo 0)" "$out"
  [[ -x "$(installed_pkg "$proj")/vendor/living-docs" ]] && ok=1
  check "binary-executable-in-package" "$ok"
  version_out="$(cd "$proj" && npx --no-install living-docs --version 2>&1)"
  check "bin-prints-asset-output" "$([[ "$version_out" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$version_out"
  end_case
}

case_bad_checksum() {
  begin_case bad || return 1
  local proj out rc leftover
  proj="$(new_project bad)"
  out="$(cd "$proj" && npm install "$TARBALL" 2>&1)"
  rc=$?
  check "install-exit-nonzero" "$([[ $rc != 0 ]] && echo 1 || echo 0)" "$out"
  leftover="$(find "$(installed_pkg "$proj")" -path '*/vendor/*' -type f 2>/dev/null)"
  check "no-binary-or-partial-file" "$([[ -z "$leftover" ]] && echo 1 || echo 0)" "$leftover"
  check "names-install-sh-fallback" "$(grep -q 'install.sh' <<<"$out" && echo 1 || echo 0)" "$out"
  end_case
}

case_ignore_scripts() {
  begin_case good || return 1
  local proj out rc first second
  proj="$(new_project ignore)"
  out="$(cd "$proj" && npm install --ignore-scripts "$TARBALL" 2>&1)"
  rc=$?
  check "install-exit-0" "$([[ $rc == 0 ]] && echo 1 || echo 0)" "$out"
  check "no-binary-after-install" "$([[ ! -e "$(installed_pkg "$proj")/vendor/living-docs" ]] && echo 1 || echo 0)"
  first="$(cd "$proj" && npx --no-install living-docs --version 2>/dev/null)"
  check "first-run-prints-version" "$([[ "$first" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$first"
  second="$(cd "$proj" && npx --no-install living-docs --version 2>/dev/null)"
  check "second-run-prints-version" "$([[ "$second" == "$FAKE_OUTPUT" ]] && echo 1 || echo 0)" "$second"
  check "one-asset-fetch-total" "$([[ "$(asset_fetch_count)" == 1 ]] && echo 1 || echo 0)" "$(cat "$ACCESS_LOG")"
  end_case
}

case_unsupported_platform() {
  begin_case good || return 1
  local proj out rc target missing=0
  proj="$(new_project unsupported)"
  out="$(cd "$proj" && LIVING_DOCS_PLATFORM="win32-x64" npm install "$TARBALL" 2>&1)"
  rc=$?
  check "install-exit-nonzero" "$([[ $rc != 0 ]] && echo 1 || echo 0)" "$out"
  for target in aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
    grep -qF "$target" <<<"$out" || missing=1
  done
  check "lists-four-supported-targets" "$([[ $missing == 0 ]] && echo 1 || echo 0)" "$out"
  end_case
}

CASES=(good_checksum bad_checksum ignore_scripts unsupported_platform)
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
