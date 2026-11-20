#!/usr/bin/env bash
#
# common.sh — builders and assertions shared by the scenario scripts. Source it from a
# scenario's setup.sh or expect.sh; both run with the scratch repository as the working
# directory and the binary under test first on PATH as `living-docs`.

BASELINE_TAG="eval-baseline"

eval_start() {
	mkdir -p docs
	living-docs manage install hooks >/dev/null
}

eval_record_number() { # eval_record_number <json from write new>
	local file
	file="$(jq -r .path <<<"$1")"
	file="$(basename "$file")"
	printf '%s' "${file%%-*}"
}

eval_adr() { # eval_adr <title> <description> <decision>: an accepted ADR with a complete body. Prints NNNN.
	local created number
	created="$(
		cat <<EOF | living-docs write new adr "$1" --description "$2" --body - --json 2>/dev/null
## Context

The team needs to settle this and write the answer down so nobody reopens it.

## Decision

We will $3.

## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later means a new decision.
EOF
	)"
	number="$(eval_record_number "$created")"
	living-docs write set "adr/$number" status Accepted >/dev/null
	printf '%s' "$number"
}

eval_issue() { # eval_issue <title> <description>; the issue body on stdin. Prints NNNN.
	local created
	created="$(living-docs write new issue "$1" --description "$2" --body - --json 2>/dev/null)"
	eval_record_number "$created"
}

eval_seal() {
	git add -A
	git commit -q --no-verify -m "scenario baseline"
	git tag "$BASELINE_TAG"
}

EXPECT_FAILED=0

eval_fail() {
	printf '%s\n' "$*"
	EXPECT_FAILED=1
}

eval_finish() {
	exit "$EXPECT_FAILED"
}

eval_record_file() { # eval_record_file <TYPE/NNNN>, TYPE being the directory name: prints the record's path
	find docs -name "${1##*/}-*.md" -path "*/${1%%/*}/*" | head -n 1
}

eval_field() { # eval_field <file> <frontmatter key>
	awk -v key="$2" '
		NR == 1 && $0 != "---" { exit }
		NR > 1 && $0 == "---" { exit }
		index($0, key ":") == 1 { sub(/^[^:]*: */, ""); print; exit }
	' "$1"
}

eval_section() { # eval_section <file> <heading line>
	awk -v heading="$2" '
		found && /^#+ / { exit }
		$0 == heading { found = 1; next }
		found
	' "$1"
}

eval_untouched() { # eval_untouched <path>: unchanged since the scenario baseline
	git diff --quiet "$BASELINE_TAG" -- "$1" && [[ -z "$(git ls-files --others --exclude-standard -- "$1")" ]]
}

eval_check_green() {
	living-docs check --json 2>/dev/null | jq -e '.ok == true' >/dev/null
}

eval_listed_with_successor() { # prints a line per in-force record whose predecessor is also in force
	local records
	records="$(living-docs read records --json 2>/dev/null)"
	jq -r '[.records[] | .number] as $listed
		| .records[] | select(.lineage != null)
		| (.lineage | capture("supersedes (?<old>[0-9]+)") // null) as $m
		| select($m != null and ((($m.old | tonumber)) as $old | $listed | index($old) != null))
		| "\(.number) is listed together with the record it supersedes"' <<<"$records"
}
