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
#         lint-docs.sh --ratchet <baseline-ref> [BUNDLE_ROOT]
#         lint-docs.sh --help
#
# Default (whole-bundle) mode:
#   Exit 0 = clean, 1 = ANY violation, 2 = usage / bundle-not-found error.
#
# --ratchet <baseline-ref> mode (diff-aware):
#   Lint the current bundle AND the bundle as of <baseline-ref> (a git ref), then
#   exit 1 ONLY for violations present now that were ABSENT at baseline (NEW debt
#   introduced by the change). Pre-existing violations are printed as informational
#   "(pre-existing)" lines and do NOT fail the run — legacy debt is not held against
#   the change. This mirrors the diff-aware ratchet the repo's other gates use:
#   only NEW violations block; pre-existing debt is grandfathered.
#
#   These checks are whole-bundle (index reachability, orphan membership, link
#   resolution, supersede integrity) — they are NOT file-local, so "lint only the
#   touched files" cannot work. The ratchet therefore compares violation SETS, not
#   files. The baseline bundle is materialized with `git worktree add` (robust for
#   a whole directory) and removed afterward. Baseline ref absent / not a git repo /
#   bundle missing at baseline → the baseline set is treated as EMPTY, so every
#   current violation is considered NEW (fail-closed, documented in --help).

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
  $PROG [BUNDLE_ROOT]               Lint the bundle (default: ./docs)
  $PROG --ratchet <ref> [BUNDLE]    Diff-aware: fail only on NEW violations vs <ref>
  $PROG --help                      Show this help

Checks (mechanical invariants only):
  * frontmatter      every non-reserved .md opens with frontmatter + non-empty type
  * index format     index.md / log.md carry no frontmatter (bundle-root index.md
                     may declare okf_version)
  * directory index  every concept file is listed in its own directory's index.md
  * reachable        every directory index.md is reachable from the bundle-root index.md
  * links resolve    every local (/… or relative) markdown link points at a file
  * supersede        a 'status: Superseded' record carries a resolvable superseded_by

Diff-aware ratchet (--ratchet <baseline-ref>):
  Lints the current bundle and the bundle as of <baseline-ref> (a git ref), and
  fails (exit 1) ONLY for violations present now that were absent at the baseline —
  i.e. NEW debt introduced by the change. Pre-existing violations are printed as
  "(pre-existing)" and do NOT fail the run; a change that adds no new violation
  passes even if the bundle carries legacy debt. The baseline is materialized with
  'git worktree add'. If <baseline-ref> is absent, the tree is not a git repo, or
  the bundle does not exist at the baseline, the baseline is treated as empty
  (every current violation counts as NEW — fail-closed).

  These checks are whole-bundle, not file-local, so the ratchet compares violation
  SETS rather than touched files. One CLI enforced at the git boundary (pre-commit)
  and in CI (a reusable Action) is harness-agnostic: it covers every AI harness AND
  human commits with one mechanism, instead of a per-harness plugin.

Exit: 0 clean (or no new violations) · 1 violations (or new violations) · 2 usage error.
EOF
}

# --- argument parsing -------------------------------------------------------

RATCHET=0
BASELINE_REF=""

case "${1:-}" in
	-h | --help)
		usage
		exit 0
		;;
	--ratchet)
		RATCHET=1
		BASELINE_REF="${2:-}"
		if [[ -z "$BASELINE_REF" ]]; then
			echo "$PROG: --ratchet requires a <baseline-ref> argument" >&2
			echo "       e.g. $PROG --ratchet HEAD docs" >&2
			exit 2
		fi
		BUNDLE="${3:-docs}"
		;;
	*)
		BUNDLE="${1:-docs}"
		;;
esac

BUNDLE="${BUNDLE%/}"

if [[ ! -d "$BUNDLE" ]]; then
	echo "$PROG: bundle root not found: $BUNDLE" >&2
	echo "       run from the repo root, or pass the docs directory: $PROG path/to/docs" >&2
	exit 2
fi

# --- violation collection ---------------------------------------------------
#
# report() does double duty: it prints the human-readable line (as before) and,
# when capturing for the ratchet, appends a NORMALIZED line to VIOL_LINES so the
# same violation matches across two different worktrees. Normalization strips the
# bundle-root prefix from the file path, so 'docs/adr/x.md' and
# '/tmp/wt.../docs/adr/x.md' compare equal — the comparison key is
# "<relpath-within-bundle>\t<message>".

VIOLATIONS=0
QUIET=""
declare -a VIOL_LINES=()
report() {
	# report <file> <message>
	[[ "$QUIET" != "quiet" ]] && printf '  %-44s %s\n' "$1" "$2"
	VIOLATIONS=$((VIOLATIONS + 1))
	# normalized key: drop the bundle prefix so paths are bundle-relative
	local rel="$1"
	rel="${rel#"$BUNDLE"/}"
	rel="${rel#"$BUNDLE"}"
	VIOL_LINES+=("$rel	$2")
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

# --- the lint body, as a function so it can run against two trees ------------
#
# lint_run <bundle> [quiet]
#   Lints <bundle>, appending normalized violations to the (caller-owned)
#   VIOL_LINES array and bumping VIOLATIONS. When the second arg is "quiet" the
#   per-violation human lines and the "bundle: …" header are suppressed (used for
#   the baseline tree in ratchet mode — we only need its violation SET). Resets
#   VIOLATIONS / VIOL_LINES at entry so it is safe to call twice.

lint_run() { # lint_run <bundle> [quiet]
	BUNDLE="${1%/}"
	QUIET="${2:-}"
	VIOLATIONS=0
	VIOL_LINES=()

	local ALL_MD ALL_INDEX ROOT_INDEX
	mapfile -t ALL_MD < <(find "$BUNDLE" -type f -name '*.md' | sort)
	ROOT_INDEX="$BUNDLE/index.md"

	if [[ "$QUIET" != "quiet" ]]; then
		echo "Living Docs lint — bundle: $BUNDLE"
		echo
	fi

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

	LAST_MD_COUNT="${#ALL_MD[@]}"
}

# --- driver -----------------------------------------------------------------

if ((RATCHET == 0)); then
	# Default whole-bundle mode: lint the current tree, fail on ANY violation.
	lint_run "$BUNDLE"
	echo
	if ((VIOLATIONS == 0)); then
		echo "OK — ${LAST_MD_COUNT} docs, no invariant violations."
		exit 0
	else
		echo "FAIL — $VIOLATIONS violation(s) across ${LAST_MD_COUNT} docs."
		exit 1
	fi
fi

# --- ratchet mode -----------------------------------------------------------
#
# Compare the current bundle's violation SET against the baseline tree's set.
# Only violations that are NEW (present now, absent at baseline) fail the run.

# 1. lint the CURRENT bundle (verbose) and snapshot its normalized set
lint_run "$BUNDLE"
CUR_COUNT="$LAST_MD_COUNT"
declare -a CUR_VIOL=("${VIOL_LINES[@]}")

# 2. materialize the baseline tree and lint it (quiet) for its set
declare -a BASE_VIOL=()
BASELINE_NOTE=""
WORKTREE=""

cleanup_worktree() {
	if [[ -n "$WORKTREE" && -d "$WORKTREE" ]]; then
		git -C "$REPO_TOPLEVEL" worktree remove --force "$WORKTREE" >/dev/null 2>&1 ||
			rm -rf "$WORKTREE" 2>/dev/null
	fi
}
trap cleanup_worktree EXIT

REPO_TOPLEVEL="$(git rev-parse --show-toplevel 2>/dev/null || true)"

if [[ -z "$REPO_TOPLEVEL" ]]; then
	BASELINE_NOTE="not a git repository — baseline treated as empty (all violations are NEW)"
elif ! git -C "$REPO_TOPLEVEL" rev-parse --verify --quiet "$BASELINE_REF^{commit}" >/dev/null 2>&1; then
	BASELINE_NOTE="baseline ref '$BASELINE_REF' not found — baseline treated as empty (all violations are NEW)"
else
	# Where does BUNDLE live relative to the repo root? (so we can find it in the worktree)
	BUNDLE_ABS="$(cd "$BUNDLE" && pwd)"
	BUNDLE_REL="${BUNDLE_ABS#"$REPO_TOPLEVEL"/}"
	if [[ "$BUNDLE_REL" == "$BUNDLE_ABS" ]]; then
		# bundle is outside the repo — cannot map into the worktree
		BASELINE_NOTE="bundle is outside the git repo — baseline treated as empty (all violations are NEW)"
	else
		WORKTREE="$(mktemp -d "${TMPDIR:-/tmp}/lint-docs-baseline.XXXXXX")"
		if git -C "$REPO_TOPLEVEL" worktree add --quiet --detach "$WORKTREE" "$BASELINE_REF" >/dev/null 2>&1; then
			BASE_BUNDLE="$WORKTREE/$BUNDLE_REL"
			if [[ -d "$BASE_BUNDLE" ]]; then
				lint_run "$BASE_BUNDLE" quiet
				BASE_VIOL=("${VIOL_LINES[@]}")
			else
				BASELINE_NOTE="bundle '$BUNDLE_REL' did not exist at '$BASELINE_REF' — baseline treated as empty (all violations are NEW)"
			fi
		else
			BASELINE_NOTE="could not check out '$BASELINE_REF' — baseline treated as empty (all violations are NEW)"
		fi
	fi
fi

# 3. set difference: NEW = CUR \ BASE ; pre-existing = CUR ∩ BASE
declare -A BASE_SET=()
for v in "${BASE_VIOL[@]:-}"; do
	[[ -n "$v" ]] && BASE_SET["$v"]=1
done

declare -a NEW_VIOL=()
declare -a OLD_VIOL=()
for v in "${CUR_VIOL[@]:-}"; do
	[[ -z "$v" ]] && continue
	if [[ -n "${BASE_SET[$v]:-}" ]]; then
		OLD_VIOL+=("$v")
	else
		NEW_VIOL+=("$v")
	fi
done

# 4. report
echo
echo "Ratchet — baseline: $BASELINE_REF"
[[ -n "$BASELINE_NOTE" ]] && echo "  note: $BASELINE_NOTE"
echo

fmt_viol() { # fmt_viol <normalized-line> <tag>
	# normalized line is "<relpath>\t<message>"; print it human-readably with a tag
	local line="$1" tag="$2" rel msg
	rel="${line%%$'\t'*}"
	msg="${line#*$'\t'}"
	printf '  %-7s %-40s %s\n' "$tag" "$rel" "$msg"
}

if ((${#OLD_VIOL[@]} > 0)); then
	echo "Pre-existing violations (grandfathered — do NOT block this change):"
	for v in "${OLD_VIOL[@]}"; do fmt_viol "$v" "[old]"; done
	echo
fi

if ((${#NEW_VIOL[@]} > 0)); then
	echo "NEW violations introduced by this change (these block):"
	for v in "${NEW_VIOL[@]}"; do fmt_viol "$v" "[NEW]"; done
	echo
	echo "FAIL — ${#NEW_VIOL[@]} new violation(s) vs baseline $BASELINE_REF (${#OLD_VIOL[@]} pre-existing, not counted)."
	exit 1
else
	echo "OK — no new violations vs baseline $BASELINE_REF (${#OLD_VIOL[@]} pre-existing, grandfathered)."
	exit 0
fi
