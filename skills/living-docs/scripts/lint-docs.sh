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

Assumptions / limitations (this is a deterministic checker over a documented input
shape, not a general markdown/YAML validator):
Requirements (parsing is delegated to mature tools, not hand-rolled):
  * lychee   link validity — every markdown link form (inline, titled, angle-bracket,
             reference-style), code-fence aware, run with --offline (no network)
  * yq       frontmatter — mikefarah/yq v4, real YAML via --front-matter=extract
             (quotes, inline comments and block scalars are all parsed correctly)
  * jq       parses lychee's JSON report

Notes:
  * link validity covers local files only; external http(s) links are not verified.
  * the OKF structural graph (directory-index membership + index reachability) is
    built from the inline links in index.md files.

Exit: 0 clean · 1 violations · 2 usage error.
EOF
}

# --- required external tools ------------------------------------------------

require_tools() {
	local missing=()
	command -v yq >/dev/null 2>&1 || missing+=("yq (mikefarah v4)")
	command -v lychee >/dev/null 2>&1 || missing+=("lychee")
	command -v jq >/dev/null 2>&1 || missing+=("jq")
	if ((${#missing[@]} > 0)); then
		echo "$PROG: missing required tool(s): ${missing[*]}" >&2
		echo "       lychee: https://lychee.cli.rs · yq v4: https://github.com/mikefarah/yq · jq: https://jqlang.github.io/jq" >&2
		exit 2
	fi
	if ! yq --version 2>&1 | grep -q mikefarah; then
		echo "$PROG: 'yq' must be mikefarah/yq v4 (found: $(yq --version 2>&1 | head -1))." >&2
		echo "       this checker uses its --front-matter=extract — install from https://github.com/mikefarah/yq" >&2
		exit 2
	fi
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

require_tools

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

fm_value() { # fm_value <file> <key>  → prints the top-level frontmatter scalar, or empty
	# Real YAML parsing via mikefarah yq: quotes, inline comments, and block scalars are
	# all handled correctly; a missing or null key prints empty so the caller reports it.
	# Callers guard with has_frontmatter, so the document always has a frontmatter block.
	yq --front-matter=extract "(.${2} // \"\")" "$1" 2>/dev/null
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
	target="${target#"${target%%[![:space:]]*}"}"   # trim leading whitespace
	if [[ "$target" == "<"* ]]; then
		# angle-bracket target: the URL is between < and the first > (a title may follow)
		target="${target#<}"
		target="${target%%>*}"
	else
		# inline target: the URL ends at the first whitespace; drop an optional "title"/'title'
		target="${target%%[[:space:]]*}"
	fi
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

# NOTE: link *validity* is delegated to lychee (see the link check below). The two helpers
# here exist only to build the OKF *structural graph* — directory-index membership and
# index→index reachability — from the inline links in index.md files.
#
# Strip fenced code blocks (``` / ~~~ regions) so example links shown inside them are not
# mistaken for live links. Indented (4-space) code blocks are out of scope.
strip_fences() { # strip_fences <file>
	awk '
		/^[[:space:]]*(```|~~~)/ { in_fence = !in_fence; next }
		!in_fence { print }
	' "$1"
}

# Print every inline markdown link target in <file> (the bit inside the parentheses),
# with fenced code blocks excluded. Reference-style [x][ref] links are NOT extracted
# (see --help "Assumptions").
links_in() { # links_in <file>
	strip_fences "$1" | grep -oE '\]\([^)]+\)' 2>/dev/null | sed -E 's/^\]\(//; s/\)$//'
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

# --- check: every local link resolves (delegated to lychee) -----------------
# lychee parses every markdown link form (inline / titled / angle-bracket / reference),
# skips fenced code blocks, and with --offline checks only local files (no network).
# --root-dir lets bundle-relative '/foo' links resolve against the bundle root.

if ((${#ALL_MD[@]} > 0)); then
	abs_bundle="$(cd "$BUNDLE" && pwd)"
	ly_json="$(lychee --offline --no-progress --format json \
		--root-dir "$abs_bundle" "${ALL_MD[@]}" 2>/dev/null || true)"
	while IFS=$'\t' read -r src url; do
		[[ -z "$src" ]] && continue
		src_rel="${src#"$PWD"/}"
		tgt="${url#file://}"
		tgt="${tgt#"$abs_bundle"/}"
		report "$src_rel" "broken link → $tgt"
	done < <(printf '%s' "$ly_json" | jq -r '
		(.error_map // {}) | to_entries[] | .key as $src
		| .value[] | [$src, .url] | @tsv' 2>/dev/null)
fi

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
