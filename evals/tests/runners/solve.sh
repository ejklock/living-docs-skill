#!/usr/bin/env bash
#
# solve.sh — a scripted stand-in for an agent. Solves the scenario named by
# LIVING_DOCS_EVAL_SCENARIO with the CLI only and prints a Claude Code style JSONL transcript
# of the commands it ran, so the scorer has something to read. The task on stdin is ignored.

set -uo pipefail

cat >/dev/null
calls=0

say() {
	jq -nc --arg text "$1" '{type: "assistant", message: {role: "assistant", content: [{type: "text", text: $text}]}}'
}

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

record_path() { # record_path <type directory>/<NNNN>
	find docs -name "${1##*/}-*.md" -path "*/${1%%/*}/*" | head -n 1
}

adr_body() { # adr_body <decision sentence>
	cat <<EOF
## Context

The team met, weighed the options and wants the answer written down.

## Decision

We will $1.

## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later needs a new decision.
EOF
}

new_adr() { # new_adr <title> <description> <decision sentence>
	step "living-docs write new adr \"$1\" --description \"$2\" --body - <<'EOF'
$(adr_body "$3")
EOF"
}

record_decision() {
	step 'living-docs read records --topic reporting'
	new_adr "Keep the reporting store on PostgreSQL" "Reports stay on PostgreSQL because three services write at once." "keep the reporting store on PostgreSQL, since SQLite's single writer would queue three services"
	step 'living-docs write set adr/0002 status Accepted'
}

supersede_adr() {
	step 'living-docs read records --links-to adr/0001'
	new_adr "Keep sessions in node memory" "Each app node holds its own sessions behind sticky routing." "hold sessions in each app node's memory behind sticky routing"
	step 'living-docs write set adr/0002 status Accepted'
	step 'living-docs write supersede adr/0001 adr/0002'
}

close_issue_keep_decision() {
	step 'living-docs read records --links-to issue/0001'
	step 'living-docs write set issue/0001 status closed'
	step "{ awk '/^### Decision$/ {f = 1; next} f && /^#+ / {exit} f && NF' $(record_path issues/0001); echo '- The export separates fields with semicolons so Excel in European locales opens it cleanly.'; } | living-docs write set issue/0001 section Decision --from -"
}

revive_superseded_adr() {
	step 'living-docs read records --links-to adr/0001'
	say 'That record cannot be revived through the CLI: a supersede has no undo, so I left the bundle as it was.'
}

legacy_callout_wording() {
	step 'living-docs check --plain'
	step 'living-docs manage fmt docs'
	step 'living-docs manage index adr'
}

rename_source_path() {
	step 'living-docs read records --topic cache'
	step "sed -i 's#src/cache\\.rs#src/store/cache.rs#g' $(record_path adr/0001)"
}

ambiguous_record_number() {
	step 'living-docs write set issue/0001 status closed'
}

answer_from_existing_decision() {
	step 'living-docs read records --topic database --full'
	say 'Tell them the reports live in PostgreSQL because three services write to them at the same time.'
}

scenario="${LIVING_DOCS_EVAL_SCENARIO:-}"
case "$scenario" in
record-decision | supersede-adr | close-issue-keep-decision | revive-superseded-adr | legacy-callout-wording | rename-source-path | ambiguous-record-number | answer-from-existing-decision)
	"${scenario//-/_}"
	;;
*)
	echo "solve.sh: no solution for scenario '$scenario'" >&2
	exit 2
	;;
esac
step 'living-docs check'
