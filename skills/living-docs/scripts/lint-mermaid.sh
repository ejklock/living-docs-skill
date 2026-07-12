#!/usr/bin/env bash
#
# lint-mermaid.sh — validate every fenced ```mermaid block with the real Mermaid parser.
#
# lint-docs.sh checks the Living Docs invariants (frontmatter, links, indexing); it never
# parses the *content* of a diagram. A ```mermaid fence with broken syntax renders as an
# error box in every consumer (GitHub, mermaid.live, the docs site) and nothing here would
# have caught it. This script closes that gap by running each extracted block through the
# upstream mermaid-cli Docker image (the real parser, not a hand-rolled grammar check).
#
# Usage:  lint-mermaid.sh [PATH...]      (default: sweep git-tracked *.md in the repo)
#         lint-mermaid.sh --help
#
# Exit:   0 = every diagram parses, 1 = at least one diagram failed, 2 = usage / tool error.

set -euo pipefail

PROG="${0##*/}"

if [[ -z "${BASH_VERSINFO:-}" || "${BASH_VERSINFO[0]}" -lt 4 ]]; then
	echo "$PROG requires bash 4+ (you have ${BASH_VERSION:-unknown}). On macOS: brew install bash." >&2
	exit 2
fi

MERMAID_CLI_IMAGE="${MERMAID_CLI_IMAGE:-minlag/mermaid-cli:11.4.2}"

usage() {
	cat <<EOF
$PROG — validate every fenced \`\`\`mermaid block with the real Mermaid parser.

Usage:
  $PROG [PATH...]    Validate diagrams in PATH(s) (files or directories).
                     With no PATH, sweep git-tracked *.md across the repo.
  $PROG --help       Show this help

Requirements:
  * docker    the CLI must be installed and the daemon reachable
              image (pinned): $MERMAID_CLI_IMAGE — override via MERMAID_CLI_IMAGE

Exit: 0 all diagrams parse · 1 at least one failed · 2 usage / tool error.
EOF
}

case "${1:-}" in
	-h | --help)
		usage
		exit 0
		;;
esac

require_tools() {
	command -v docker >/dev/null 2>&1 || {
		echo "$PROG: missing required tool: docker" >&2
		echo "       install: https://docs.docker.com/get-docker/" >&2
		exit 2
	}
	if ! docker info >/dev/null 2>&1; then
		echo "$PROG: docker CLI found but the daemon is unreachable — is Docker running?" >&2
		exit 2
	fi
}

# discover_files <path...>  → NUL-delimited *.md paths.
# With explicit PATH args, sweep exactly what was named (files or directories).
# With no args, sweep git-tracked markdown across the whole repo; fall back to a
# plain find outside a git repo.
discover_files() {
	local -a paths=("$@")
	if ((${#paths[@]} > 0)); then
		find "${paths[@]}" -type f -name '*.md' -print0
		return
	fi
	if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
		# Exclude the fixtures dir from the default sweep only: 11-mermaid-invalid is an
		# intentionally-broken diagram, and tests/run.sh already covers it via explicit paths.
		git ls-files -z -- '*.md' ':!skills/living-docs/tests/fixtures'
	else
		find . -type f -name '*.md' -print0
	fi
}

# collect_candidates <path...>  → NUL-delimited files that actually contain a mermaid fence
# (a fast grep pre-filter so the awk extraction pass only touches relevant files).
collect_candidates() {
	local f
	while IFS= read -r -d '' f; do
		grep -Fq -- '```mermaid' "$f" 2>/dev/null && printf '%s\0' "$f"
	done < <(discover_files "$@")
}

# extract_diagrams <scratch_dir> <manifest_file> <file...>
# Awk state machine: captures every block opened by an optional-indent ```mermaid line
# and closed by the next optional-indent ``` line. Each block is written to
# <scratch_dir>/NNN.mmd; the manifest gets one 'NNN<TAB>file<TAB>startline' line per block.
extract_diagrams() {
	local scratch="$1" manifest="$2"
	shift 2
	awk -v scratch="$scratch" -v manifest="$manifest" '
		FNR == 1 { in_block = 0 }
		!in_block && /^[[:space:]]*```mermaid[[:space:]]*$/ {
			in_block = 1
			startline = FNR
			buffer = ""
			next
		}
		in_block && /^[[:space:]]*```[[:space:]]*$/ {
			count++
			id = sprintf("%03d", count)
			outfile = scratch "/" id ".mmd"
			printf "%s", buffer > outfile
			close(outfile)
			printf "%s\t%s\t%s\n", id, FILENAME, startline >> manifest
			in_block = 0
			next
		}
		in_block { buffer = buffer $0 "\n" }
	' "$@"
}

# awk treats a bare "name=value" argument as a variable assignment rather than a file to
# read. Prefix relative paths with "./" so no tracked filename can ever be misread that way.
awk_safe_path() {
	case "$1" in
		/* | ./*) printf '%s' "$1" ;;
		*) printf './%s' "$1" ;;
	esac
}

# validate_diagram <scratch_dir> <id> <file> <startline>  → 0 valid, 1 invalid (prints FAIL)
validate_diagram() {
	local scratch="$1" id="$2" file="$3" startline="$4" err
	if err="$(docker run --rm -u "$(id -u):$(id -g)" -v "$scratch:/data" \
		"$MERMAID_CLI_IMAGE" -i "/data/$id.mmd" -o "/data/out/$id.svg" -q 2>&1)"; then
		return 0
	fi
	echo "FAIL $file:$startline"
	printf '%s\n' "$err" | head -5 | sed 's/^/    /'
	return 1
}

require_tools

SCRATCH="$(mktemp -d)"
trap 'rm -rf "$SCRATCH"' EXIT
mkdir -p "$SCRATCH/out"
MANIFEST="$SCRATCH/manifest.tsv"
: >"$MANIFEST"

candidates=()
while IFS= read -r -d '' f; do
	candidates+=("$f")
done < <(collect_candidates "$@")

if ((${#candidates[@]} > 0)); then
	awk_files=()
	for f in "${candidates[@]}"; do
		awk_files+=("$(awk_safe_path "$f")")
	done
	extract_diagrams "$SCRATCH" "$MANIFEST" "${awk_files[@]}"
fi

diagram_count=0
file_count=0
failed=0

if [[ -s "$MANIFEST" ]]; then
	diagram_count="$(wc -l <"$MANIFEST" | tr -d '[:space:]')"
	file_count="$(cut -f2 "$MANIFEST" | sort -u | wc -l | tr -d '[:space:]')"
	while IFS=$'\t' read -r num file startline; do
		validate_diagram "$SCRATCH" "$num" "$file" "$startline" || failed=$((failed + 1))
	done <"$MANIFEST"
fi

echo
if ((failed > 0)); then
	echo "FAIL: $failed of $diagram_count diagram(s) failed to parse."
	exit 1
fi
echo "OK: $diagram_count diagram(s) across $file_count file(s)."
