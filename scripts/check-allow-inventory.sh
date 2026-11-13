#!/usr/bin/env bash
#
# check-allow-inventory.sh — enforce the shrink-only allow-inventory gate.
#
# Every #[allow(clippy::too_many_lines)] annotation is inventoried in
# scripts/allow-inventory-baseline.txt at the count measured when the lint was enabled. A
# file may remove annotations (shrink toward zero) but never gain one: a file not on the
# baseline that carries an annotation fails, and a listed file whose count grows past its
# baseline fails. Once a listed file's count reaches zero, the function was fixed — the
# baseline entry is stale and must be removed, so a zero count still fails until then. A
# baseline entry for a file that no longer exists is stale and fails.
#
# .claude/worktrees/ holds other agents' parallel git worktrees; their .rs files are
# excluded the same way target/ is (see check-file-size.sh for the same rationale); the scan
# runs from the root over relative paths, so a checkout's own location (under .claude/worktrees/,
# under a target/ directory, or a path holding a glob character) never changes the verdict.
#
# Usage:  check-allow-inventory.sh
# Exit:   0 = no violations, 1 = at least one violation, or find could not read part of the tree.

set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
baseline_file="$root/scripts/allow-inventory-baseline.txt"
marker='#[allow(clippy::too_many_lines)]'

fail=0
declare -A baseline_counts=()
declare -A baseline_seen=()

load_baseline() {
	local line count path
	while IFS= read -r line || [[ -n "$line" ]]; do
		[[ -z "$line" ]] && continue
		count="${line%% *}"
		path="${line#* }"
		baseline_counts["$path"]="$count"
	done <"$baseline_file"
}

report() { # report <message>
	printf '%s\n' "$1"
	fail=1
}

count_allows() { # count_allows <file> -> prints the marker count
	grep -Fc -- "$marker" "$1" || true
}

check_grandfathered_file() { # check_grandfathered_file <path> <count> <baseline>
	local path="$1" count="$2" baseline="$3"
	if ((count == 0)); then
		report "STALE BASELINE: $path now has zero allows — remove the entry — the file leaves the list permanently"
		return
	fi
	if ((count > baseline)); then
		report "GREW: $path has $count allow(s), exceeds its baseline of $baseline"
		return
	fi
	if ((count < baseline)); then
		printf 'ADVISORY: %s shrank to %s allow(s) (baseline %s) — tighten the baseline entry\n' "$path" "$count" "$baseline"
	fi
}

check_current_file() { # check_current_file <path> <count>
	local path="$1" count="$2"
	if [[ -v baseline_counts["$path"] ]]; then
		baseline_seen["$path"]=1
		check_grandfathered_file "$path" "$count" "${baseline_counts[$path]}"
		return
	fi
	if ((count > 0)); then
		report "NEW ALLOW: $path has $count allow(s) but is not in the baseline"
	fi
}

check_stale_entries() {
	local path
	for path in "${!baseline_counts[@]}"; do
		[[ -v baseline_seen["$path"] ]] && continue
		report "STALE BASELINE: $path no longer exists"
	done
}

listing=""

enumerate_and_check() {
	local rel count
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
		count="$(count_allows "$root/$rel")"
		check_current_file "$rel" "$count"
	done <"$listing"
}

load_baseline
enumerate_and_check
check_stale_entries

if [[ "$fail" -ne 0 ]]; then
	echo "Allow-inventory gate FAILED."
	exit 1
fi
echo "Allow-inventory gate OK: ${#baseline_counts[@]} tracked file(s)."
