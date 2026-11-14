#!/usr/bin/env bash
#
# install.sh — bootstrap the living-docs CLI binary.
#
# Packaging is the CLI (ADR 0028): the release binary is the unit of
# distribution and this script's only job is to get it onto PATH. Skill
# placement is a CLI verb (`living-docs manage install skills --harness
# <claude|opencode|codex|pi> [--project] [--dir]`) and enforcement is another
# (`living-docs manage install hooks`) — see README.md → Installation.
#
# Usage:
#   ./install.sh [cli] [options]
#
# Target (default and only supported target: cli):
#   cli   download the living-docs binary (release asset, sha256-verified,
#         cargo-build fallback) to --dir (default ~/.local/bin)
#
# Options:
#   --dir <path>     override the destination directory (default ~/.local/bin)
#   --uninstall      remove a previous living-docs CLI install
#   --from-source    skip the release asset, build with `cargo build --release`
#   -n, --dry-run    print what would happen, change nothing
#   -h, --help       show this help
#
# Environment:
#   LIVING_DOCS_VERSION   pin an exact release tag instead of resolving latest
#
# Examples:
#   ./install.sh                       # download the CLI to ~/.local/bin
#   ./install.sh --dir /usr/local/bin  # install to a custom directory
#   ./install.sh --from-source         # build the CLI with cargo instead
#   ./install.sh --uninstall           # remove a previous install

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

UNINSTALL=0
DRYRUN=0
FROM_SOURCE=0
OVERRIDE_DIR=""
TARGET=""

CLI_REPO="ejklock/living-docs-skill"

log()  { printf '%s\n' "$*"; }
run()  { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] $*"; else eval "$*"; fi; }
note() { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] would install: $*"; else log "installed: $*"; fi; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() { sed -n '2,32p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    cli) TARGET="cli" ;;
    --uninstall) UNINSTALL=1 ;;
    -n|--dry-run) DRYRUN=1 ;;
    --from-source) FROM_SOURCE=1 ;;
    --dir) shift; OVERRIDE_DIR="${1:-}"; [[ -n "$OVERRIDE_DIR" ]] || die "--dir needs a path" ;;
    -h|--help) usage; exit 0 ;;
    --*) die "unknown option: $1 (try --help)" ;;
    *) die "unsupported target: $1 (cli is the only target; try --help)" ;;
  esac
  shift
done
TARGET="${TARGET:-cli}"

cli_target_triple() {
  local os="$1" arch="$2" os_part arch_part
  case "$os" in
    Darwin) os_part="apple-darwin" ;;
    Linux)  os_part="unknown-linux-gnu" ;;
    *) return 1 ;;
  esac
  case "$arch" in
    arm64|aarch64) arch_part="aarch64" ;;
    x86_64|amd64)  arch_part="x86_64" ;;
    *) return 1 ;;
  esac
  printf '%s-%s\n' "$arch_part" "$os_part"
}

cli_verify_sha256() {
  local file="$1" sumfile="$2" expected actual
  expected="$(awk '{print $1}' "$sumfile")"
  if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "$file" | awk '{print $1}')"
  else
    actual="$(shasum -a 256 "$file" | awk '{print $1}')"
  fi
  [[ -n "$expected" && "$expected" == "$actual" ]]
}

build_cli_from_source() {
  local dest="$1"
  command -v cargo >/dev/null 2>&1 \
    || die "cargo not found; install Rust or drop --from-source once a release asset exists"
  run "cargo build --release --manifest-path '$SCRIPT_DIR/cli/Cargo.toml'"
  run "mkdir -p '$dest'"
  run "install -m 755 '$SCRIPT_DIR/target/release/living-docs' '$dest/living-docs'"
  note "living-docs (built from source) -> $dest/living-docs"
}

cli_resolve_tag() {
  local pinned="${LIVING_DOCS_VERSION:-}"
  if [[ -n "$pinned" ]]; then
    case "$pinned" in
      v*) printf '%s\n' "$pinned" ;;
      *)  printf 'v%s\n' "$pinned" ;;
    esac
    return 0
  fi

  local latest_url="https://api.github.com/repos/$CLI_REPO/releases/latest"
  local tag
  tag="$(curl -fsSL "$latest_url" 2>/dev/null | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]*)".*/\1/')"
  [[ -n "$tag" ]] || return 1
  printf '%s\n' "$tag"
}

install_cli() {
  local dest="${OVERRIDE_DIR:-$HOME/.local/bin}"
  local bin_path="$dest/living-docs"

  if [[ $UNINSTALL -eq 1 ]]; then
    run "rm -f '$bin_path'"
    log "uninstalled: $bin_path"
    return
  fi

  if [[ $FROM_SOURCE -eq 1 ]]; then
    build_cli_from_source "$dest"
    return
  fi

  local triple asset tag base asset_url sha_url tmp
  if ! triple="$(cli_target_triple "$(uname -s)" "$(uname -m)")"; then
    log "unsupported platform ($(uname -s)/$(uname -m)) for a prebuilt binary; building from source"
    build_cli_from_source "$dest"
    return
  fi

  asset="living-docs-$triple"
  if ! tag="$(cli_resolve_tag)"; then
    log "could not resolve a release tag (set LIVING_DOCS_VERSION or check network); building from source"
    build_cli_from_source "$dest"
    return
  fi
  base="https://github.com/$CLI_REPO/releases/download/$tag"
  asset_url="$base/$asset"
  sha_url="$asset_url.sha256"

  if [[ $DRYRUN -eq 1 ]]; then
    log "  [dry-run] would download $asset_url ($tag) -> $bin_path (sha256-verified)"
    return
  fi

  tmp="$(mktemp -d)"
  if curl -fsSL -o "$tmp/$asset" "$asset_url" 2>/dev/null \
      && curl -fsSL -o "$tmp/$asset.sha256" "$sha_url" 2>/dev/null \
      && cli_verify_sha256 "$tmp/$asset" "$tmp/$asset.sha256"; then
    run "mkdir -p '$dest'"
    run "install -m 755 '$tmp/$asset' '$bin_path'"
    note "living-docs ($triple) -> $bin_path"
    rm -rf "$tmp"
    return
  fi

  rm -rf "$tmp"
  log "release asset unavailable for $triple; falling back to build from source"
  build_cli_from_source "$dest"
}

log "living-docs installer — target: $TARGET$([[ $UNINSTALL -eq 1 ]] && echo ' [uninstall]')$([[ $DRYRUN -eq 1 ]] && echo ' [dry-run]')"
log ""
install_cli
