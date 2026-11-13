#!/usr/bin/env bash
#
# check-file-size.sh — enforce the 300-line file-size ratchet (CLAUDE.md rule 5).
#
# Every .rs file must stay at or under 300 lines. A file already over the limit when the
# ratchet was introduced is grandfathered in scripts/file-size-baseline.txt at its measured
# line count — it may shrink freely but never grow past that baseline, and once it drops to
# 300 lines or fewer the entry must be removed (the file leaves the list permanently, it
# never re-enters). A baseline entry for a file that no longer exists is stale and fails.
#
# .claude/worktrees/ holds other agents' parallel git worktrees (separate branches, sharing
# this repo's objects). Their .rs files are not part of this checkout's canonical tree, so
# counting them would make the gate's outcome depend on which worktrees happen to exist at
# run time — the opposite of deterministic. The same holds for where the checkout itself lives:
# its own location (under .claude/worktrees/, under a target/ directory, or a path holding a
# glob character) never changes the verdict. The scan therefore runs from the root over relative
# paths and prunes only the root's own .claude/worktrees, target and .git.
#
# Usage:  check-file-size.sh
# Exit:   0 = no violations, 1 = at least one violation, or find could not read part of the tree.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
baseline_file="$root/scripts/file-size-baseline.txt"
limit=300

fail=0
declare -A baseline_lines=()
declare -A baseline_seen=()

load_baseline() {
	local line count path
	while IFS= read -r line || [[ -n "$line" ]]; do
		[[ -z "$line" ]] && continue
		count="${line%% *}"
		path="${line#* }"
		baseline_lines["$path"]="$count"
	done <"$baseline_file"
}

report() { # report <message>
	printf '%s\n' "$1"
	fail=1
}

check_grandfathered_file() { # check_grandfathered_file <path> <lines> <baseline>
	local path="$1" lines="$2" baseline="$3"
	if ((lines <= limit)); then
		report "STALE BASELINE: $path is now $lines lines (limit $limit) — remove the entry — the file leaves the list permanently"
		return
	fi
	if ((lines > baseline)); then
		report "GREW: $path has $lines lines, exceeds its baseline of $baseline"
		return
	fi
	if ((lines < baseline)); then
		printf 'ADVISORY: %s shrank to %s lines (baseline %s) — tighten the baseline entry\n' "$path" "$lines" "$baseline"
	fi
}

check_current_file() { # check_current_file <path> <lines>
	local path="$1" lines="$2"
	if [[ -v baseline_lines["$path"] ]]; then
		baseline_seen["$path"]=1
		check_grandfathered_file "$path" "$lines" "${baseline_lines[$path]}"
		return
	fi
	if ((lines > limit)); then
		report "OVER LIMIT: $path has $lines lines (limit $limit, not grandfathered)"
	fi
}

check_stale_entries() {
	local path
	for path in "${!baseline_lines[@]}"; do
		[[ -v baseline_seen["$path"] ]] && continue
		report "STALE BASELINE: $path no longer exists"
	done
}

listing=""

enumerate_and_check() {
	local rel lines
	listing="$(mktemp)"
	trap 'rm -f "$listing"' EXIT
	# a process substitution's exit status is ignored, so find runs to a file whose status is checked
	if ! (cd "$root" && find . \
		\( -path ./.claude/worktrees -o -path ./target -o -path ./.git \) -prune -o \
		-name '*.rs' -print0) >"$listing"; then
		echo "cannot enumerate .rs files: find failed to read part of the tree"
		exit 1
	fi
	while IFS= read -r -d '' rel; do
		rel="${rel#./}"
		lines="$(wc -l <"$root/$rel" | tr -d '[:space:]')"
		check_current_file "$rel" "$lines"
	done <"$listing"
}

load_baseline
enumerate_and_check
check_stale_entries

if [[ "$fail" -ne 0 ]]; then
	echo "File-size ratchet FAILED."
	exit 1
fi
echo "File-size ratchet OK: ${#baseline_lines[@]} grandfathered file(s), limit ${limit} lines."
