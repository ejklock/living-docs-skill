#!/usr/bin/env bash
#
# install.sh — install the Living Docs skill bundle into an AI agent tool.
#
# Living Docs is plain markdown. "Installing" means copying the three skills
# (living-docs, okf-knowledge-format, research-artifacts) to where your tool
# discovers instructions, and — for tools that need it — generating a small
# rule/instruction pointer file.
#
# Usage:
#   ./install.sh [harness] [options]
#
# Harness (default: claude):
#   claude     ~/.claude/skills            (or .claude/skills with --project)
#   opencode   ~/.config/opencode/skills   (or .opencode/skills with --project)
#   pi         ~/.pi/agent/skills          (or .pi/skills with --project)
#   cursor     .cursor/rules/*.mdc                       (always project-scoped)
#   copilot    .github/instructions/*.instructions.md    (always project-scoped)
#   all        install every harness above
#   pocock     git clone Matt Pocock's companion skills (grill-me, to-prd, to-issues)
#
# Options:
#   --project        install into the current project, not the global user dir
#   --dir <path>     override the destination skills directory (claude/opencode/pi)
#   --uninstall      remove a previous Living Docs install for the harness
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

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SKILLS=(living-docs okf-knowledge-format research-artifacts)

PROJECT=0
UNINSTALL=0
DRYRUN=0
OVERRIDE_DIR=""
HARNESS=""

log()  { printf '%s\n' "$*"; }
run()  { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] $*"; else eval "$*"; fi; }
note() { if [[ $DRYRUN -eq 1 ]]; then log "  [dry-run] would install: $*"; else log "installed: $*"; fi; }
die()  { printf 'ERROR: %s\n' "$*" >&2; exit 1; }

usage() { sed -n '2,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

# --- parse args ---
while [[ $# -gt 0 ]]; do
  case "$1" in
    claude|opencode|pi|cursor|copilot|all|pocock) HARNESS="$1" ;;
    --project)   PROJECT=1 ;;
    --uninstall) UNINSTALL=1 ;;
    -n|--dry-run) DRYRUN=1 ;;
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
    run "cp -R '$SCRIPT_DIR/skills/$s' '$dest/$s'"
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

do_harness() {
  case "$1" in
    claude)   install_skills_dir "$HOME/.claude/skills" ".claude/skills" ;;
    opencode) install_skills_dir "$HOME/.config/opencode/skills" ".opencode/skills" ;;
    pi)       install_skills_dir "$HOME/.pi/agent/skills" ".pi/skills" ;;
    cursor)   install_cursor ;;
    copilot)  install_copilot ;;
    pocock)   install_pocock ;;
  esac
}

log "Living Docs installer — harness: $HARNESS$([[ $PROJECT -eq 1 ]] && echo ' (project)')$([[ $UNINSTALL -eq 1 ]] && echo ' [uninstall]')$([[ $DRYRUN -eq 1 ]] && echo ' [dry-run]')"
log ""
if [[ "$HARNESS" == "all" ]]; then
  for h in claude opencode pi cursor copilot; do log "── $h ──"; do_harness "$h"; log ""; done
else
  do_harness "$HARNESS"
fi
