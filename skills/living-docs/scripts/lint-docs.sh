#!/usr/bin/env bash
#
# lint-docs.sh — mechanically validate the Living Docs invariants on a docs bundle.
#
# The five invariants are a contract. Three of them are mechanical and should be
# checked by a machine, not by an agent re-reading prose ("a constraint without an
# instrument is a vibe"). This script is that instrument. It checks:
#
#   - Indexed or it doesn't exist (invariant 3): every concept file is listed in its
#     directory's index.md, and every directory index is reachable from the bundle root.
#   - Links resolve (invariants 2/3): every bundle-relative / relative link to a local
#     file points at a file that exists.
#   - Supersede, never rewrite (invariant 4): a Superseded record carries superseded_by,
#     and the target exists.
#   - OKF format: every non-reserved doc opens with frontmatter carrying a non-empty
#     `type`; index.md/log.md carry no frontmatter (except the bundle-root index.md,
#     which may declare okf_version).
#
# Invariants 1 (docs-first mirror) and 2's "one home per fact" semantic half are NOT
# mechanical — they stay with the reviewing agent. This script never claims to check them.
#
# Usage:  lint-docs.sh [BUNDLE_ROOT]      (default: docs)
#         lint-docs.sh --help
#
# Exit:   0 = clean, 1 = violations found, 2 = usage / bundle-not-found error.

set -uo pipefail

if [[ -z "${BASH_VERSINFO:-}" || "${BASH_VERSINFO[0]}" -lt 4 ]]; then
	echo "lint-docs.sh requires bash 4+ (you have ${BASH_VERSION:-unknown}). On macOS: brew install bash." >&2
	exit 2
fi

PROG="${0##*/}"

usage() {
	cat <<EOF
$PROG — validate the mechanical Living Docs invariants on a docs bundle.

Usage:
  $PROG [BUNDLE_ROOT]    Lint the bundle (default: ./docs)
  $PROG --help           Show this help

Checks (mechanical invariants only):
  * frontmatter      every non-reserved .md opens with frontmatter + non-empty type
  * index format     index.md / log.md carry no frontmatter (bundle-root index.md
                     may declare okf_version)
  * directory index  every concept file is listed in its own directory's index.md
  * reachable        every directory index.md is reachable from the bundle-root index.md
  * links resolve    every local (/… or relative) markdown link points at a file
  * supersede        a 'status: Superseded' record carries a resolvable superseded_by

Exit: 0 clean · 1 violations · 2 usage error.
EOF
}

case "${1:-}" in
	-h | --help)
		usage
		exit 0
		;;
esac

BUNDLE="${1:-docs}"
BUNDLE="${BUNDLE%/}"

if [[ ! -d "$BUNDLE" ]]; then
	echo "$PROG: bundle root not found: $BUNDLE" >&2
	echo "       run from the repo root, or pass the docs directory: $PROG path/to/docs" >&2
	exit 2
fi

VIOLATIONS=0
report() {
	# report <file> <message>
	printf '  %-44s %s\n' "$1" "$2"
	VIOLATIONS=$((VIOLATIONS + 1))
}

# --- helpers ----------------------------------------------------------------

is_reserved() { # is_reserved <basename>
	[[ "$1" == "index.md" || "$1" == "log.md" ]]
}

has_frontmatter() { # has_frontmatter <file>  → 0 if first line is '---'
	[[ "$(head -n 1 "$1")" == "---" ]]
}

fm_value() { # fm_value <file> <key>  → prints trimmed value within frontmatter block
	awk -v key="$2" '
		NR == 1 && $0 != "---" { exit }
		NR == 1 { infm = 1; next }
		infm && $0 == "---" { exit }
		infm {
			if ($0 ~ "^" key ":") {
				sub("^" key ":[[:space:]]*", "")
				gsub(/^[[:space:]]+|[[:space:]]+$/, "")
				gsub(/^"|"$/, "")
				print
				exit
			}
		}
	' "$1"
}

# Normalize a path: collapse '.' and '..' segments. Pure bash (portable).
normpath() { # normpath <path>  → prints normalized path
	local path="$1" abs="" seg out=()
	[[ "$path" == /* ]] && abs="/"
	local IFS=/
	for seg in $path; do
		case "$seg" in
			"" | .) ;;
			..)
				if ((${#out[@]} > 0)) && [[ "${out[$((${#out[@]} - 1))]}" != ".." ]]; then
					unset "out[$((${#out[@]} - 1))]"
				elif [[ -z "$abs" ]]; then
					out+=("..")
				fi
				;;
			*) out+=("$seg") ;;
		esac
	done
	local joined="${out[*]}"
	printf '%s%s\n' "$abs" "$joined"
}

# Resolve a markdown link target (as written in <file>) to a filesystem path,
# or print nothing if the link is external / a pure anchor / unsupported.
resolve_link() { # resolve_link <file> <target>
	local file="$1" target="$2"
	target="${target%%#*}"      # drop anchor
	[[ -z "$target" ]] && return        # pure anchor → in-doc, skip
	case "$target" in
		*://* | mailto:* | tel:*) return ;;   # external
	esac
	if [[ "$target" == /* ]]; then
		# bundle-relative
		normpath "$BUNDLE/$target"
	else
		normpath "$(dirname "$file")/$target"
	fi
}

# Print every markdown link target in <file> (the bit inside the parentheses).
links_in() { # links_in <file>
	grep -oE '\]\([^)]+\)' "$1" 2>/dev/null | sed -E 's/^\]\(//; s/\)$//'
}

# --- collect the corpus -----------------------------------------------------

mapfile -t ALL_MD < <(find "$BUNDLE" -type f -name '*.md' | sort)

ROOT_INDEX="$BUNDLE/index.md"

echo "Living Docs lint — bundle: $BUNDLE"
echo

# --- check 1: bundle-root index exists --------------------------------------

if [[ ! -f "$ROOT_INDEX" ]]; then
	report "$ROOT_INDEX" "missing bundle-root index.md (invariant 3)"
fi

# --- per-file checks: frontmatter / type / index format ---------------------

for f in "${ALL_MD[@]}"; do
	base="${f##*/}"
	if is_reserved "$base"; then
		# index.md / log.md: no frontmatter, except bundle-root index.md (okf_version)
		if has_frontmatter "$f"; then
			if [[ "$f" == "$ROOT_INDEX" ]]; then
				okf="$(fm_value "$f" okf_version)"
				[[ -z "$okf" ]] && report "$f" "bundle-root index.md frontmatter lacks okf_version"
			else
				report "$f" "$base must not carry frontmatter (OKF §6)"
			fi
		fi
		continue
	fi
	# non-reserved concept file: must have frontmatter with a non-empty type
	if ! has_frontmatter "$f"; then
		report "$f" "missing OKF frontmatter (needs a non-empty 'type')"
		continue
	fi
	type_val="$(fm_value "$f" type)"
	if [[ -z "$type_val" ]]; then
		report "$f" "frontmatter has no non-empty 'type'"
	fi
done

# --- check: every concept file is listed in its directory's index.md --------
# The bundle-root constitution.md is the root of trace and is deliberately NOT indexed.

for f in "${ALL_MD[@]}"; do
	base="${f##*/}"
	is_reserved "$base" && continue
	[[ "$f" == "$BUNDLE/constitution.md" ]] && continue

	dir="$(dirname "$f")"
	dir_index="$dir/index.md"
	if [[ ! -f "$dir_index" ]]; then
		report "$f" "no index.md in its directory ($dir) — orphan (invariant 3)"
		continue
	fi
	listed=0
	while IFS= read -r tgt; do
		resolved="$(resolve_link "$dir_index" "$tgt")"
		[[ -z "$resolved" ]] && continue
		if [[ "$(normpath "$f")" == "$resolved" ]]; then
			listed=1
			break
		fi
	done < <(links_in "$dir_index")
	((listed == 0)) && report "$f" "not listed in $dir_index — orphan (invariant 3)"
done

# --- check: every directory index.md is reachable from the bundle root ------
# BFS from the root index over index→index (and index→dir) links.

if [[ -f "$ROOT_INDEX" ]]; then
	mapfile -t ALL_INDEX < <(find "$BUNDLE" -type f -name 'index.md' | sort)
	declare -A REACHED=()
	queue=("$(normpath "$ROOT_INDEX")")
	REACHED["$(normpath "$ROOT_INDEX")"]=1
	while ((${#queue[@]} > 0)); do
		cur="${queue[0]}"
		queue=("${queue[@]:1}")
		[[ -f "$cur" ]] || continue
		while IFS= read -r tgt; do
			resolved="$(resolve_link "$cur" "$tgt")"
			[[ -z "$resolved" ]] && continue
			# a link may point at a directory (→ its index.md) or directly at an index.md
			if [[ -d "$resolved" ]]; then
				resolved="$(normpath "$resolved/index.md")"
			fi
			[[ "${resolved##*/}" == "index.md" ]] || continue
			if [[ -z "${REACHED[$resolved]:-}" ]]; then
				REACHED["$resolved"]=1
				queue+=("$resolved")
			fi
		done < <(links_in "$cur")
	done
	for idx in "${ALL_INDEX[@]}"; do
		n="$(normpath "$idx")"
		[[ -z "${REACHED[$n]:-}" ]] && report "$idx" "directory index not reachable from $ROOT_INDEX (invariant 3)"
	done
fi

# --- check: every local link resolves ---------------------------------------

for f in "${ALL_MD[@]}"; do
	while IFS= read -r tgt; do
		resolved="$(resolve_link "$f" "$tgt")"
		[[ -z "$resolved" ]] && continue
		if [[ ! -e "$resolved" ]]; then
			report "$f" "broken link → $tgt"
		fi
	done < <(links_in "$f")
done

# --- check: supersede integrity (invariant 4) -------------------------------

for f in "${ALL_MD[@]}"; do
	base="${f##*/}"
	is_reserved "$base" && continue
	has_frontmatter "$f" || continue
	status="$(fm_value "$f" status)"
	# case-insensitive compare (portable to bash without ${,,})
	status_lc="$(printf '%s' "$status" | tr '[:upper:]' '[:lower:]')"
	if [[ "$status_lc" == "superseded" ]]; then
		sb="$(fm_value "$f" superseded_by)"
		if [[ -z "$sb" ]]; then
			report "$f" "status: Superseded but superseded_by is empty (invariant 4)"
			continue
		fi
		# resolve superseded_by (an NNNN) to a sibling file with that number prefix
		dir="$(dirname "$f")"
		if ! compgen -G "$dir/${sb}-*.md" >/dev/null && [[ ! -f "$dir/${sb}.md" ]]; then
			report "$f" "superseded_by: $sb has no matching record in $dir (invariant 4)"
		fi
	fi
done

# --- verdict ----------------------------------------------------------------

echo
if ((VIOLATIONS == 0)); then
	echo "OK — ${#ALL_MD[@]} docs, no invariant violations."
	exit 0
else
	echo "FAIL — $VIOLATIONS violation(s) across ${#ALL_MD[@]} docs."
	exit 1
fi
