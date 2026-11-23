#!/usr/bin/env bash
#
# lib.sh — setup and expectation of the scale scenarios. The generator copies it next to
# common.sh in the scenarios directory; a scenario's setup.sh and expect.sh source it and call
# scale_setup or scale_expect with their own directory. Both run inside the scratch repository
# with the binary under test first on PATH.
#
# A scenario directory holds manifest.tsv (see manifest.awk) and scenario.env (TYPE, SIZE,
# WORDING, POSITION, ARM, PLANT: the planted record's number, 0 for the control).
#
# expect prints one line per finding, each with a prefix the aggregator reads:
#   missed:      the planted record is not caught
#   false-alarm: another record is newly superseded, or listed in CONFLICTS.txt (prompted)
#   incomplete:  the decision is not recorded, check is red, or CONFLICTS.txt is absent

. "$(dirname "${BASH_SOURCE[0]}")/common.sh"

SCALE_ADR_DIR="docs/adr"

scale_padded() { printf '%04d' "$((10#$1))"; }

scale_adr() { # scale_adr <number> <title> <description> <context> <decision>
	local created
	created="$(
		cat <<EOF | living-docs write new adr "$2" --description "$3" --body - --json 2>/dev/null
## Context

$4

## Decision

We will $5.

## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later means a new decision.
EOF
	)"
	[[ "$(eval_record_number "$created")" == "$(scale_padded "$1")" ]] || {
		echo "scale: ADR '$2' did not get number $1" >&2
		return 1
	}
	living-docs write set "adr/$(scale_padded "$1")" status Accepted >/dev/null
}

scale_apply() { # scale_apply <op> <a> <b> <c> <d> <e>: one manifest line
	case "$1" in
	adr) scale_adr "$2" "$3" "$4" "$5" "$6" ;;
	supersede) living-docs write supersede "adr/$(scale_padded "$2")" "adr/$(scale_padded "$3")" >/dev/null ;;
	deprecate) living-docs write set "adr/$(scale_padded "$2")" status Deprecated >/dev/null ;;
	*) echo "scale: unknown manifest operation $1" >&2 && return 1 ;;
	esac
}

scale_setup() { # scale_setup <scenario dir>
	local op a b c d e
	eval_start
	while IFS=$'\t' read -r op a b c d e; do
		scale_apply "$op" "$a" "$b" "$c" "$d" "$e" || return 1
	done <"$1/manifest.tsv"
	eval_check_green || { echo "scale: the generated bundle fails check" >&2 && return 1; }
	eval_seal
}

scale_status_at_baseline() { # scale_status_at_baseline <path>
	git show "$BASELINE_TAG:$1" 2>/dev/null | awk 'NR > 1 && $0 == "---" { exit } /^status:/ { sub(/^status: */, ""); print; exit }'
}

scale_newly_superseded() { # prints the number of every ADR Superseded now and not at the baseline
	local file
	for file in "$SCALE_ADR_DIR"/[0-9][0-9][0-9][0-9]-*.md; do
		[[ -f "$file" && "$(eval_field "$file" status)" == Superseded ]] || continue
		[[ "$(scale_status_at_baseline "$file")" == Superseded ]] && continue
		basename "$file" | cut -c1-4 | sed -E 's/^0+([0-9])/\1/'
	done
}

scale_new_records() { # prints the path of every record added since the baseline
	{
		git diff --name-only --diff-filter=A "$BASELINE_TAG" -- docs
		git ls-files --others --exclude-standard -- docs
	} | grep -E '\.md$' | grep -v -E '(^|/)index\.md$' | sort -u
}

scale_linked() { # scale_linked <number>: a record added since the baseline names it
	local pattern file
	pattern="(^|[^0-9])$(scale_padded "$1")([^0-9]|$)|[Aa][Dd][Rr][Ss]?[ /#-]*0*$1([^0-9]|$)"
	while IFS= read -r file; do
		[[ -f "$file" ]] && grep -q -E "$pattern" "$file" && return 0
	done < <(scale_new_records)
	return 1
}

scale_listed() { # prints the record number each CONFLICTS.txt line starts with, after an optional bullet and adr/ or ADR prefix; a line starting with text lists nothing
	[[ -f CONFLICTS.txt ]] || return 0
	sed -n -E 's/^[[:space:]]*([-*][[:space:]]+)?([Aa][Dd][Rr][ \/#-]*)?([0-9]+).*/\3/p' CONFLICTS.txt | sed -E 's/^0+([0-9])/\1/' | sort -un
}

scale_new_adr_in_force() {
	local file status
	while IFS= read -r file; do
		[[ "$file" == "$SCALE_ADR_DIR"/* && -f "$file" ]] || continue
		status="$(eval_field "$file" status)"
		[[ "$status" != Superseded && "$status" != Deprecated ]] && return 0
	done < <(scale_new_records)
	return 1
}

scale_judge_unprompted() { # scale_judge_unprompted <plant number>
	local plant="$1" n caught=0
	for n in $(scale_newly_superseded); do
		if [[ "$n" == "$plant" ]]; then caught=1; else eval_fail "false-alarm: adr/$(scale_padded "$n") was superseded"; fi
	done
	((plant == 0 || caught == 1)) && return
	scale_linked "$plant" || eval_fail "missed: adr/$(scale_padded "$plant") is neither superseded nor named by a new record"
}

scale_judge_prompted() { # scale_judge_prompted <plant number>
	local plant="$1" n caught=0
	[[ -f CONFLICTS.txt ]] || eval_fail "incomplete: no CONFLICTS.txt"
	for n in $(scale_listed); do
		if [[ "$n" == "$plant" ]]; then caught=1; else eval_fail "false-alarm: $n is listed in CONFLICTS.txt"; fi
	done
	for n in $(scale_newly_superseded); do
		[[ "$n" == "$plant" ]] || eval_fail "false-alarm: adr/$(scale_padded "$n") was superseded"
	done
	((plant == 0 || caught == 1)) || eval_fail "missed: adr/$(scale_padded "$plant") is not listed in CONFLICTS.txt"
}

scale_expect() { # scale_expect <scenario dir>
	local TYPE SIZE WORDING POSITION ARM PLANT
	. "$1/scenario.env"
	if [[ "$ARM" == prompted ]]; then scale_judge_prompted "$PLANT"; else scale_judge_unprompted "$PLANT"; fi
	scale_new_adr_in_force || eval_fail "incomplete: no new ADR in force records the decision"
	eval_check_green || eval_fail "incomplete: living-docs check is red"
	eval_finish
}
