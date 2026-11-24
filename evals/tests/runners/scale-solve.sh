#!/usr/bin/env bash
#
# scale-solve.sh — a scripted stand-in for an agent on the scale scenarios. Usage: scale-solve.sh MODE,
# MODE being catch (handles the planted decision correctly), miss (records the new decision and
# ignores the planted one) or wrong (touches a record that has nothing to do with the decision).
# Reads the scenario name from LIVING_DOCS_EVAL_SCENARIO and prints a Claude Code style JSONL
# transcript of the commands it ran. The task on stdin is ignored.

set -uo pipefail

cat >/dev/null
MODE="${1:?mode}"
calls=0

step() {
	calls=$((calls + 1))
	local id="call_$calls" out rc=0 failed=false
	jq -nc --arg id "$id" --arg command "$1" \
		'{type: "assistant", message: {role: "assistant", content: [{type: "tool_use", id: $id, name: "Bash", input: {command: $command}}]}}'
	out="$(bash -c "$1" 2>&1)" || rc=$?
	((rc == 0)) || failed=true
	jq -nc --arg id "$id" --arg content "$out" --argjson failed "$failed" \
		'{type: "user", message: {role: "user", content: [{type: "tool_result", tool_use_id: $id, content: $content, is_error: $failed}]}}'
}

readonly ADR_DIR="docs/adr"

padded() { printf '%04d' "$1"; }

planted_number() { # planted_number <size>: 0 for a control
	case "$LIVING_DOCS_EVAL_SCENARIO" in
	control-*) echo 0 ;;
	*-early-*) echo 3 ;;
	*-late-*) echo $(($1 - 2)) ;;
	esac
}

first_other_accepted() { # first_other_accepted <plant number>
	local file number
	for file in "$ADR_DIR"/[0-9]*.md; do
		number=$((10#$(basename "$file" | cut -c1-4)))
		((number == $1)) && continue
		grep -q '^status: Accepted$' "$file" && { echo "$number"; return; }
	done
}

record_decision() { # record_decision <number of the new record>
	step "living-docs write new adr 'Keep user sessions on each node' --description 'Each node holds its own sessions.' --body - <<'EOF'
## Context

The team met and wants the answer written down.

## Decision

We will keep user sessions in each app node's memory behind sticky routing.

## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later needs a new decision.
EOF"
	step "living-docs write set adr/$(padded "$1") status Accepted"
}

unprompted() { # unprompted <plant> <new number>
	local target=""
	case "$MODE" in
	catch) (($1 > 0)) && target="$1" ;;
	wrong) target="$(first_other_accepted "$1")" ;;
	esac
	record_decision "$2"
	[[ -z "$target" ]] || step "living-docs write supersede adr/$(padded "$target") adr/$(padded "$2")"
}

prompted() { # prompted <plant> <new number>
	record_decision "$2"
	case "$MODE" in
	catch) if (($1 > 0)); then step "printf '%s\n' $1 > CONFLICTS.txt"; else step ': > CONFLICTS.txt'; fi ;;
	wrong) step "printf '%s\n' $(first_other_accepted "$1") > CONFLICTS.txt" ;;
	*) step ': > CONFLICTS.txt' ;;
	esac
}

size="$(find "$ADR_DIR" -name '[0-9]*.md' | wc -l | tr -d ' ')"
plant="$(planted_number "$size")"
if [[ "$LIVING_DOCS_EVAL_SCENARIO" == *-prompted ]]; then prompted "$plant" $((size + 1)); else unprompted "$plant" $((size + 1)); fi
step 'living-docs check'
