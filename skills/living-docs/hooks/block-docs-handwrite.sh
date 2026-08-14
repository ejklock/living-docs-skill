#!/usr/bin/env bash
#
# block-docs-handwrite.sh — PreToolUse gate enforcing CLI-owned doc authoring
# (ADR 0019 block rules, ADR 0020 scope, ADR 0021 in-repo distribution).
#
# Reads the Claude Code PreToolUse payload (JSON) on stdin. Inside the
# CLI-owned type directories of the docs bundle (adr|bdr|prd|issues|research)
# it blocks:
#   - a Write creating a new NNNN-*.md record    -> `living-docs new`
#   - any direct write to a type index.md        -> `living-docs index`
#   - a Write/Edit/MultiEdit whose result changes a CLI-owned frontmatter key
#     (type, title, status, supersedes, superseded_by, timestamp)
#                                                -> `living-docs status`/`supersede`/`fmt`
# The frontmatter guard simulates the change and compares the CLI-owned key
# lines before vs after, so body prose, `description`, and `tags` stay freely
# editable — including CLI-owned key names quoted inside body code fences.
#
# Env:  LIVING_DOCS_ENFORCE=block|warn (default block)
#       LIVING_DOCS_BUNDLE=<dir>       (default docs)
# Exit: 0 allow (also on warn or any ambiguity — fail-open), 2 block.

set -u
LC_ALL=C
shopt -u patsub_replacement 2>/dev/null || true

MODE="${LIVING_DOCS_ENFORCE:-block}"
BUNDLE="${LIVING_DOCS_BUNDLE:-docs}"
CLI_OWNED_KEYS='type|title|status|supersedes|superseded_by|timestamp'

allow() { exit 0; }

deny() {
  printf 'living-docs: %s\n' "$1" >&2
  [ "$MODE" = "warn" ] && exit 0
  exit 2
}

deny_frontmatter() {
  deny "frontmatter keys (${CLI_OWNED_KEYS//|/, }) are CLI-owned — use \`living-docs status <NNNN> <Status>\`, \`living-docs supersede <old> <new>\`, or \`living-docs fmt\`. Edit ONLY the body below the closing ---."
}

json_field() {
  jq -r "$1 // empty" <<<"$INPUT" 2>/dev/null || printf ''
}

frontmatter_of() {
  awk 'NR==1 { if ($0 != "---") exit; next } $0 == "---" { exit } { print }' <<<"$1"
}

owned_lines_of() {
  frontmatter_of "$1" | grep -E "^(${CLI_OWNED_KEYS}):" || true
}

deny_unless_owned_lines_kept() {
  local before="$1" after="$2"
  [ "$(owned_lines_of "$before")" = "$(owned_lines_of "$after")" ] || deny_frontmatter
}

apply_edit() {
  local content="$1" old="$2" new="$3" replace_all="$4"
  if [ "$replace_all" = "true" ]; then
    printf '%s' "${content//"$old"/$new}"
  else
    printf '%s' "${content/"$old"/$new}"
  fi
}

guard_write() {
  if [ ! -e "$FILE" ]; then
    deny "records are scaffolded by the CLI — run \`living-docs new <adr|bdr|prd|issue|research|view> \"<title>\"\` (numbering + frontmatter + skeleton), then write ONLY the body below the closing ---. Binary missing? \`make build\`."
  fi
  deny_unless_owned_lines_kept "$(cat "$FILE")" "$(json_field '.tool_input.content')"
}

guard_edit() {
  local content old new
  old="$(json_field '.tool_input.old_string')"
  new="$(json_field '.tool_input.new_string')"
  [ -n "$old" ] || return 0
  content="$(cat "$FILE" 2>/dev/null)" || return 0
  case "$content" in *"$old"*) ;; *) return 0 ;; esac
  deny_unless_owned_lines_kept "$content" \
    "$(apply_edit "$content" "$old" "$new" "$(json_field '.tool_input.replace_all')")"
}

guard_multi_edit() {
  local content updated count i old new
  content="$(cat "$FILE" 2>/dev/null)" || return 0
  updated="$content"
  count="$(jq '.tool_input.edits | length' <<<"$INPUT" 2>/dev/null)"
  case "$count" in '' | *[!0-9]*) return 0 ;; esac
  i=0
  while [ "$i" -lt "$count" ]; do
    old="$(json_field ".tool_input.edits[$i].old_string")"
    new="$(json_field ".tool_input.edits[$i].new_string")"
    if [ -n "$old" ]; then
      updated="$(apply_edit "$updated" "$old" "$new" "$(json_field ".tool_input.edits[$i].replace_all")")"
    fi
    i=$((i + 1))
  done
  deny_unless_owned_lines_kept "$content" "$updated"
}

command -v jq >/dev/null 2>&1 || allow
INPUT="$(cat 2>/dev/null)" || allow
TOOL="$(json_field '.tool_name')"
FILE="$(json_field '.tool_input.file_path')"
[ -n "$FILE" ] || allow

[[ "$FILE" =~ (^|/)"$BUNDLE"/(adr|bdr|prd|issues|research|architecture)/([^/]+)$ ]] || allow
DIR="${BASH_REMATCH[2]}"
NAME="${BASH_REMATCH[3]}"

if [ "$NAME" = "index.md" ]; then
  deny "type indexes are generated — run \`living-docs index\` instead of writing $NAME by hand."
fi

if [ "$DIR" = "architecture" ]; then
  [[ "$NAME" =~ ^[A-Za-z0-9][A-Za-z0-9_-]*\.md$ ]] || allow
else
  [[ "$NAME" =~ ^[0-9]{4}-.*\.md$ ]] || allow
fi

case "$TOOL" in
Write) guard_write ;;
Edit) guard_edit ;;
MultiEdit) guard_multi_edit ;;
esac

allow
