#!/usr/bin/env bash
#
# install.sh — install the Living Docs skill bundle into an AI agent tool.
#
# Living Docs is plain markdown. "Installing" means copying the three skills'
# slim SKILL.md stubs (living-docs, okf-knowledge-format, research-artifacts,
# plus okf-knowledge-format's vendored reference/) to where your tool
# discovers instructions — full rules/templates detail is served on demand by
# the `living-docs skill` CLI command, not copied to disk — and, for tools
# that need it, generating a small rule/instruction pointer file.
#
# Usage:
#   ./install.sh [harness] [options]
#
# Harness (default: claude). Native SKILL.md skills (auto-discovered):
#   claude     ~/.claude/skills            (or .claude/skills with --project)
#   opencode   ~/.config/opencode/skills   (or .opencode/skills with --project)
#   codex      ~/.codex/skills             (or .codex/skills with --project)
# Generated rule / instruction file (project-scoped):
#   cursor     .cursor/rules/*.mdc
#   copilot    .github/instructions/*.instructions.md
# AGENTS.md-based (copy + reference from AGENTS.md):
#   pi         ~/.pi/agent/skills          (or .pi/skills with --project)
#   all        install every harness above
#   pocock     git clone Matt Pocock's companion skills (grill-me, to-prd, to-issues)
#   cli        install the living-docs binary (release asset, cargo build fallback)
#
# Options:
#   --project        install into the current project, not the global user dir
#   --dir <path>     override the destination skills directory (claude/opencode/codex/pi/cli)
#   --uninstall      remove a previous Living Docs install for the harness
#   --from-source    (cli only) skip the release asset, build with `cargo build --release`
#   -n, --dry-run    print what would happen, change nothing
#   -h, --help       show this help
#
# Examples:
#   ./install.sh                      # Claude Code, global
#   ./install.sh cursor               # Cursor rules in the current project
#   ./install.sh opencode --project   # OpenCode, into ./.opencode/skills
#   ./install.sh all                  # every supported harness
#   ./install.sh pocock               # clone Matt Pocock's companion skills
#   ./install.sh claude --uninstall   # remove the global Claude install
#   ./install.sh cli                  # download the living-docs binary to ~/.local/bin
#   ./install.sh cli --from-source    # build the living-docs binary with cargo instead

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILLS=(living-docs okf-knowledge-format research-artifacts)

PROJECT=0
UNINSTALL=0
DRYRUN=0
FROM_SOURCE=0
OVERRIDE_DIR=""
HARNESS=""

CLI_REPO="ejklock/living-docs-skill"

log()  { printf '%s\n' "$*"; }
run()  { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] $*"; else eval "$*"; fi; }
note() { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] would install: $*"; else log "installed: $*"; fi; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() { sed -n '2,42p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    claude|opencode|codex|pi|cursor|copilot|all|pocock|cli) HARNESS="$1" ;;
    --project)   PROJECT=1 ;;
    --uninstall) UNINSTALL=1 ;;
    -n|--dry-run) DRYRUN=1 ;;
    --from-source) FROM_SOURCE=1 ;;
    --dir) shift; OVERRIDE_DIR="${1:-}"; [[ -n "$OVERRIDE_DIR" ]] || die "--dir needs a path" ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown argument: $1 (try --help)" ;;
  esac
  shift
done
HARNESS="${HARNESS:-claude}"

for s in "${SKILLS[@]}"; do
  [[ -d "$SCRIPT_DIR/skills/$s" ]] || die "missing skill source: skills/$s"
done

# --- copy the three skill dirs into a skills directory ---
copy_skills() {
  local dest="$1"
  if [[ $UNINSTALL -eq 1 ]]; then
    for s in "${SKILLS[@]}"; do run "rm -rf '${dest:?}/$s'"; done
    log "uninstalled skills from $dest"
    return
  fi
  run "mkdir -p '$dest'"
  for s in "${SKILLS[@]}"; do
    run "rm -rf '${dest:?}/$s'"
    run "mkdir -p '$dest/$s'"
    run "cp '$SCRIPT_DIR/skills/$s/SKILL.md' '$dest/$s/SKILL.md'"
    if [[ -d "$SCRIPT_DIR/skills/$s/reference" ]]; then
      run "cp -R '$SCRIPT_DIR/skills/$s/reference' '$dest/$s/reference'"
    fi
    note "$s -> $dest/$s"
  done
}

# --- write a project rule/instruction pointer file with a generated header ---
# $1 = target file, $2 = frontmatter header block
write_pointer() {
  local target="$1" header="$2"
  if [[ $UNINSTALL -eq 1 ]]; then run "rm -f '$target'"; log "removed $target"; return; fi
  run "mkdir -p '$(dirname "$target")'"
  if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] write $target"; return; fi
  { printf '%s\n' "$header"; cat "$SCRIPT_DIR/skills/living-docs/SKILL.md"; } > "$target"
  log "installed: living-docs rule -> $target"
}

install_skills_dir() {
  local global="$1" project="$2"
  local dest="$OVERRIDE_DIR"
  if [[ -z "$dest" ]]; then
    if [[ $PROJECT -eq 1 ]]; then dest="$project"; else dest="$global"; fi
  fi
  copy_skills "$dest"
  log "Done. Restart your session so $HARNESS picks up the skills."
}

install_cursor() {
  local target=".cursor/rules/living-docs.mdc"
  local header='---
description: "Living Docs — keep documentation a living system (ADR/BDR/PRD/constitution, no-drift invariants, OKF format)"
globs: "docs/**,**/*.md"
alwaysApply: false
---'
  write_pointer "$target" "$header"
  [[ $UNINSTALL -eq 1 ]] || log "Tip: the okf-knowledge-format & research-artifacts skills live in skills/ for reference."
}

install_copilot() {
  local target=".github/instructions/living-docs.instructions.md"
  local header='---
applyTo: "docs/**,**/*.md"
---'
  write_pointer "$target" "$header"
  [[ $UNINSTALL -eq 1 ]] || log "Tip: for a repo-wide rule, append the same guidance to .github/copilot-instructions.md."
}

# --- clone Matt Pocock's MIT-licensed companion skills straight from source ---
install_pocock() {
  local dest="${OVERRIDE_DIR:-$HOME/.matt-pocock-skills}"
  if [[ $UNINSTALL -eq 1 ]]; then run "rm -rf '${dest:?}'"; log "removed $dest"; return; fi
  log "Companion skills — Matt Pocock (https://github.com/mattpocock/skills, MIT)"
  if [[ $DRYRUN -eq 1 ]]; then
    log "  [dry-run] git clone --depth 1 https://github.com/mattpocock/skills.git '$dest'"
  elif [[ -d "$dest/.git" ]]; then
    run "git -C '$dest' pull --ff-only"
    log "updated: $dest"
  else
    run "git clone --depth 1 https://github.com/mattpocock/skills.git '$dest'"
    log "cloned: $dest"
  fi
  log "Next: run his 'setup-matt-pocock-skills' skill to wire grill-me / to-prd / to-issues into your tool."
  log "His repo is MIT-licensed — keep its LICENSE notice if you copy files."
}

# --- map uname -s/-m to the release asset's target triple ---
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

# --- compare a downloaded file's sha256 against a `sha256sum`-style sidecar ---
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

  local triple asset version tag base asset_url sha_url tmp
  if ! triple="$(cli_target_triple "$(uname -s)" "$(uname -m)")"; then
    log "unsupported platform ($(uname -s)/$(uname -m)) for a prebuilt binary; building from source"
    build_cli_from_source "$dest"
    return
  fi

  asset="living-docs-$triple"
  version="$(<"$SCRIPT_DIR/VERSION")"
  tag="v$version"
  base="https://github.com/$CLI_REPO/releases/download/$tag"
  asset_url="$base/$asset"
  sha_url="$asset_url.sha256"

  if [[ $DRYRUN -eq 1 ]]; then
    log "  [dry-run] would download $asset_url -> $bin_path (sha256-verified)"
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

do_harness() {
  case "$1" in
    claude)   install_skills_dir "$HOME/.claude/skills" ".claude/skills" ;;
    opencode) install_skills_dir "$HOME/.config/opencode/skills" ".opencode/skills" ;;
    codex)    install_skills_dir "$HOME/.codex/skills" ".codex/skills" ;;
    pi)       install_skills_dir "$HOME/.pi/agent/skills" ".pi/skills" ;;
    cursor)   install_cursor ;;
    copilot)  install_copilot ;;
    pocock)   install_pocock ;;
    cli)      install_cli ;;
  esac
}

log "Living Docs installer — harness: $HARNESS$([[ $PROJECT -eq 1 ]] && echo ' (project)')$([[ $UNINSTALL -eq 1 ]] && echo ' [uninstall]')$([[ $DRYRUN -eq 1 ]] && echo ' [dry-run]')"
log ""
if [[ "$HARNESS" == "all" ]]; then
  for h in claude opencode codex pi cursor copilot; do log "── $h ──"; do_harness "$h"; log ""; done
else
  do_harness "$HARNESS"
fi
