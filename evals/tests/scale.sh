#!/usr/bin/env bash
#
# scale.sh — tests for the scale scenarios: the generator, the bundles setup.sh builds, the
# unprompted and prompted scoring of expect.sh, and the aggregator. Sourced by run.sh, which
# provides TMP, HERE, EVALS, assert_eq, assert_true, run_eval and wanted. Scenarios are generated
# into $TMP; the scripted stand-in scale-solve.sh stands in for the agent.

SCALE="$EVALS/scale"
readonly SCALE_KIND=adr
readonly SCALE_NAMES="control-240 control-60 t1-240-para-early t1-240-same-late t1-60-para-late t1-60-same-early t2-240-para-late t2-240-same-early t2-60-para-early t2-60-same-late"

scale_generate() { # scale_generate <dir> [seed]; sets RC and STDERR
	STDERR="$(bash "$SCALE/generate.sh" "$@" 2>&1 >/dev/null)"
	RC=$?
}

scale_small() { # scale_small <dir>: scenarios at sizes 12 and 24
	LIVING_DOCS_SCALE_SIZES="12 24" bash "$SCALE/generate.sh" "$1" >/dev/null 2>&1
}

scale_run() { # scale_run <out> <scenarios dir> <runner> [name...]; sets RC
	local out="$1" dir="$2" runner="$3"
	shift 3
	LIVING_DOCS_EVAL_SCENARIOS="$dir" run_eval "$out" "$runner" "$@"
}

scale_names() { # scale_names <scenarios dir> <arm>
	find "$1" -mindepth 1 -maxdepth 1 -type d -name "*-$2" -exec basename {} \; | LC_ALL=C sort
}

test_scale_generator() {
	echo "scale generator"
	local a="$TMP/gen-a" b="$TMP/gen-b" c="$TMP/gen-c" name arm expected=""
	scale_generate "$a" 153
	assert_eq "seed 153 writes the scenarios" 0 "$RC"
	scale_generate "$b" 153
	diff -r "$a" "$b" >/dev/null
	assert_eq "the same seed writes the same tree" 0 "$?"
	scale_generate "$c" 154
	cmp -s "$a/t1-60-same-early-unprompted/manifest.tsv" "$c/t1-60-same-early-unprompted/manifest.tsv"
	assert_eq "another seed writes another manifest" 1 "$?"
	for name in $SCALE_NAMES; do for arm in prompted unprompted; do expected+="$name-$arm"$'\n'; done; done
	assert_eq "twenty scenario directories with the planned names" "$(LC_ALL=C sort <<<"${expected%$'\n'}")" "$(find "$a" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | LC_ALL=C sort)"
	assert_eq "every scenario has its five files" 100 \
		"$(find "$a" -mindepth 2 -maxdepth 2 \( -name task.md -o -name setup.sh -o -name expect.sh -o -name manifest.tsv -o -name scenario.env \) | wc -l | tr -d ' ')"
	assert_eq "ten prompted scenarios are written" 10 "$(scale_names "$a" prompted | wc -l | tr -d ' ')"
	assert_eq "only prompted tasks ask for CONFLICTS.txt" "$(scale_names "$a" prompted)" \
		"$(grep -l CONFLICTS.txt "$a"/*/task.md | xargs -n1 dirname | xargs -n1 basename | LC_ALL=C sort)"
}

test_scale_generator_usage() {
	echo "scale generator usage"
	local out="$TMP/gen-bad" sizes
	scale_generate "$out" abc
	assert_eq "a non-numeric seed exits 2" 2 "$RC"
	assert_true "a non-numeric seed says why" grep -q 'SEED must be a number' <<<"$STDERR"
	for sizes in "60" "60 120 240" "5 60" "60 x"; do
		LIVING_DOCS_SCALE_SIZES="$sizes" scale_generate "$out"
		assert_eq "sizes '$sizes' exit 2" 2 "$RC"
		assert_true "sizes '$sizes' say why" grep -q 'two sizes' <<<"$STDERR"
	done
	LIVING_DOCS_SCALE_SIZES="12 400" scale_generate "$out"
	assert_true "a size needing more topics than the table holds fails" test "$RC" -ne 0
	assert_true "the shortage is named" grep -q 'topics' <<<"$STDERR"
}

scale_adr_files() { # scale_adr_files <scratch>
	find "$1/docs/$SCALE_KIND" -name '[0-9]*.md' | LC_ALL=C sort
}

scale_adrs_matching() { # scale_adrs_matching <scratch> <pattern>: the ADR files whose text matches
	scale_adr_files "$1" | xargs grep -l -E "$2"
}

scale_title_of() { # scale_title_of <scratch> <number>
	awk -F': ' '$1 == "title" { print $2; exit }' "$1"/docs/"$SCALE_KIND"/"$2"-*.md
}

scale_status_of() { # scale_status_of <scratch> <number>
	awk -F': ' '$1 == "status" { print $2; exit }' "$1"/docs/"$SCALE_KIND"/"$2"-*.md
}

test_scale_bundle() {
	echo "scale bundle"
	local dir="$TMP/bundle-scenarios" out="$TMP/bundle-out" big control early scratch
	bash "$SCALE/generate.sh" "$dir" >/dev/null 2>&1
	LIVING_DOCS_EVAL_KEEP=1 scale_run "$out" "$dir" "$HERE/runners/noop.sh" t1-240-same-late-unprompted control-60-unprompted t1-60-same-early-unprompted
	big="$(jq -r .scratch "$out"/*/t1-240-same-late-unprompted/score.json)"
	control="$(jq -r .scratch "$out"/*/control-60-unprompted/score.json)"
	early="$(jq -r .scratch "$out"/*/t1-60-same-early-unprompted/score.json)"
	assert_eq "the large bundle has 240 ADRs" 240 "$(scale_adr_files "$big" | wc -l | tr -d ' ')"
	assert_eq "the large bundle passes check" true "$(jq -c .check_ok "$out"/*/t1-240-same-late-unprompted/score.json)"
	assert_eq "the late plant sits at position 238" "Keep user sessions in Redis" "$(scale_title_of "$big" 0238)"
	assert_eq "the plant is Accepted" Accepted "$(scale_status_of "$big" 0238)"
	assert_true "some ADR is Superseded" test -n "$(scale_adrs_matching "$big" '^status: Superseded$')"
	assert_true "some ADR is Deprecated" test -n "$(scale_adrs_matching "$big" '^status: Deprecated$')"
	assert_eq "no ADR points at the plant" 0 "$(scale_adrs_matching "$big" 0238 | grep -c -v '/0238-')"
	assert_eq "the control has 60 ADRs" 60 "$(scale_adr_files "$control" | wc -l | tr -d ' ')"
	assert_eq "the control passes check" true "$(jq -c .check_ok "$out"/*/control-60-unprompted/score.json)"
	assert_eq "the control has no planted title" 0 "$(scale_adrs_matching "$control" 'Keep user sessions in Redis' | wc -l | tr -d ' ')"
	assert_eq "the early plant sits at position 3" "Keep user sessions in Redis" "$(scale_title_of "$early" 0003)"
	for scratch in "$big" "$control" "$early"; do rm -rf "$scratch"; done
}

scale_bin() { # scale_bin: a directory holding the binary as living-docs
	mkdir -p "$TMP/scale-bin"
	ln -sf "$LIVING_DOCS_BIN" "$TMP/scale-bin/living-docs"
	printf '%s' "$TMP/scale-bin"
}

scale_base() { # scale_base <scenarios dir> <name>: builds a sealed scratch repository under $TMP/base-NAME
	local base="$TMP/base-$2"
	[[ -d "$base" ]] && return
	mkdir -p "$base"
	git -C "$base" init -q
	git -C "$base" config user.name Eval
	git -C "$base" config user.email eval@example.com
	(cd "$base" && PATH="$(scale_bin):$PATH" bash "$1/$2/setup.sh" >/dev/null 2>&1)
}

scale_case() { # scale_case <scenarios dir> <name>: a fresh copy of the base; prints its path
	local copy
	scale_base "$1" "$2"
	copy="$(mktemp -d "$TMP/case.XXXXXX")"
	cp -R "$TMP/base-$2/." "$copy/"
	printf '%s' "$copy"
}

scale_add_adr() { # scale_add_adr <scratch> <mention text>: a new Accepted ADR whose body holds the text
	local created number
	created="$(cd "$1" && PATH="$(scale_bin):$PATH" living-docs write new adr "Record a new decision" --description "A new decision." --json --body - 2>/dev/null <<BODY
## Context

The team decided something new. $2

## Decision

We will keep sessions on each node.

## Consequences

**Easier / gained:**
- One agreed answer.

**Harder / accepted trade-offs:**
- Changing it later needs a new decision.
BODY
)"
	number="$(basename "$(jq -r .path <<<"$created")" | cut -c1-4)"
	(cd "$1" && PATH="$(scale_bin):$PATH" living-docs write set "adr/$number" status Accepted >/dev/null)
	printf '%s' "$number"
}

scale_ld() { # scale_ld <scratch> <args...>
	local dir="$1"
	shift
	(cd "$dir" && PATH="$(scale_bin):$PATH" living-docs "$@" >/dev/null 2>&1)
}

scale_score() { # scale_score <scenarios dir> <name> <scratch>: the lines expect.sh prints; sets RC
	OUTPUT="$(cd "$3" && PATH="$(scale_bin):$PATH" bash "$1/$2/expect.sh" 2>&1)"
	RC=$?
}

scale_count() { # scale_count <prefix>: lines of OUTPUT starting with the prefix
	grep -c "^$1" <<<"$OUTPUT" || true
}

test_scale_unprompted_mentions() {
	echo "scale unprompted scoring: mentions"
	local dir="$TMP/unp-scenarios" name=t1-12-same-early-unprompted scratch mention
	scale_small "$dir"
	for mention in "See $SCALE_KIND/0003." "See ${SCALE_KIND^^} 0003." "See ${SCALE_KIND^^}-3." "See 0003-keep-user-sessions-in-redis.md."; do
		scratch="$(scale_case "$dir" "$name")"
		scale_add_adr "$scratch" "$mention" >/dev/null
		scale_score "$dir" "$name" "$scratch"
		assert_eq "a record naming '$mention' counts as a catch" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	done
	for mention in "It ran 3 times." "See 0030." "See ${SCALE_KIND^^} 0030." "See ${SCALE_KIND^^} 30."; do
		scratch="$(scale_case "$dir" "$name")"
		scale_add_adr "$scratch" "$mention" >/dev/null
		scale_score "$dir" "$name" "$scratch"
		assert_eq "'$mention' is not a catch" 1 "$(scale_count missed:)"
	done
}

test_scale_unprompted_outcomes() {
	echo "scale unprompted scoring: outcomes"
	local dir="$TMP/unp-scenarios" name=t1-12-same-early-unprompted scratch new
	scale_small "$dir"
	scratch="$(scale_case "$dir" "$name")"
	scale_score "$dir" "$name" "$scratch"
	assert_eq "no new ADR in force is incomplete" 1 "$(scale_count incomplete:)"
	assert_eq "no new ADR in force fails" 1 "$RC"
	scratch="$(scale_case "$dir" "$name")"
	new="$(scale_add_adr "$scratch" "Nothing else.")"
	scale_score "$dir" "$name" "$scratch"
	assert_eq "a miss prints missed:" 1 "$(scale_count missed:)"
	scale_ld "$scratch" write supersede adr/0003 "adr/$new"
	scale_score "$dir" "$name" "$scratch"
	assert_eq "superseding the plant is caught and clean" "0 0 0" "$(scale_count missed:) $(scale_count false-alarm:) $RC"
	scratch="$(scale_case "$dir" "$name")"
	new="$(scale_add_adr "$scratch" "Nothing else.")"
	scale_ld "$scratch" write supersede adr/0004 "adr/$new"
	scale_score "$dir" "$name" "$scratch"
	assert_eq "superseding another record is a false alarm" 1 "$(scale_count false-alarm:)"
	assert_eq "superseding another record leaves the plant missed" 1 "$(scale_count missed:)"
}

test_scale_unprompted_baseline() {
	echo "scale unprompted scoring: baseline and control"
	local dir="$TMP/unp-scenarios" name=t1-12-same-early-unprompted scratch new
	scale_small "$dir"
	scratch="$(scale_case "$dir" "$name")"
	new="$(scale_add_adr "$scratch" "Nothing else.")"
	scale_ld "$scratch" write supersede adr/0003 "adr/$new"
	git -C "$scratch" add -A
	git -C "$scratch" commit -q --no-verify -m "plant already superseded"
	git -C "$scratch" tag -f eval-baseline >/dev/null
	scale_add_adr "$scratch" "Nothing else." >/dev/null
	scale_score "$dir" "$name" "$scratch"
	assert_eq "a plant superseded at the baseline is not a catch" 1 "$(scale_count missed:)"
	scratch="$(scale_case "$dir" control-12-unprompted)"
	scale_score "$dir" control-12-unprompted "$scratch"
	assert_eq "a control with a new ADR and no supersede is clean" "0 0" "$(scale_count false-alarm:) $(scale_count missed:)"
	scale_ld "$scratch" write supersede adr/0004 "adr/$(scale_add_adr "$scratch" "Nothing else.")"
	scale_score "$dir" control-12-unprompted "$scratch"
	assert_eq "a control with any supersede is a false alarm" 1 "$(scale_count false-alarm:)"
}

scale_conflicts() { # scale_conflicts <name> <content | NONE>: scores a scratch with that CONFLICTS.txt; sets OUTPUT
	local dir="$TMP/unp-scenarios" scratch
	scratch="$(scale_case "$dir" "$1")"
	scale_add_adr "$scratch" "Nothing else." >/dev/null
	[[ "$2" == NONE ]] || printf '%s' "$2" >"$scratch/CONFLICTS.txt"
	scale_score "$dir" "$1" "$scratch"
}

test_scale_prompted() {
	echo "scale prompted scoring"
	local name=t1-12-same-early-prompted listed
	scale_small "$TMP/unp-scenarios"
	for listed in "3" "0003" "$SCALE_KIND/0003" "${SCALE_KIND^^} 0003"; do
		scale_conflicts "$name" "$listed"$'\n'
		assert_eq "listing '$listed' catches the plant" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	done
	scale_conflicts "$name" ""
	assert_eq "an empty file is a miss and not incomplete" "1 0" "$(scale_count missed:) $(scale_count incomplete:)"
	scale_conflicts "$name" NONE
	assert_eq "no file is incomplete and a miss" "1 1" "$(scale_count missed:) $(scale_count incomplete:)"
	scale_conflicts "$name" $'3\n5\n'
	assert_eq "listing the plant and another is one false alarm" "0 1" "$(scale_count missed:) $(scale_count false-alarm:)"
	assert_true "the false alarm names the other record" grep -q '^false-alarm: 5 ' <<<"$OUTPUT"
	scale_conflicts control-12-prompted $'7\n'
	assert_eq "a control listing a number is a false alarm" 1 "$(scale_count false-alarm:)"
	scale_conflicts control-12-prompted ""
	assert_eq "a control listing nothing is clean" "0 0" "$(scale_count false-alarm:) $(scale_count incomplete:)"
}

scale_modes() { # scale_modes <arm> <mode> <out>: runs the ten scenarios of an arm through the stand-in
	local dir="$TMP/modes-scenarios" names
	[[ -d "$dir" ]] || scale_small "$dir"
	mapfile -t names < <(scale_names "$dir" "$1")
	scale_run "$3" "$dir" "$HERE/runners/scale-solve.sh $2" "${names[@]}"
}

scale_expect_summary() { # scale_expect_summary <out> <prefix> <planted|control|all>: scenarios passing or printing prefix
	jq -s -c --arg prefix "$2" --arg which "$3" '[.[] | select(($which == "all") or (($which == "control") == (.scenario | startswith("control"))))
		| select(.expect_failures | any(startswith($prefix)))] | length' "$1"/*/*/score.json
}

test_scale_modes() {
	echo "scale scoring through run.sh"
	local arm out
	for arm in unprompted prompted; do
		out="$TMP/mode-catch-$arm"
		scale_modes "$arm" catch "$out"
		assert_eq "catch passes all ten $arm scenarios" 10 "$(jq -s '[.[] | select(.expect_ok)] | length' "$out"/*/*/score.json)"
		out="$TMP/mode-miss-$arm"
		scale_modes "$arm" miss "$out"
		assert_eq "miss fails the eight planted $arm scenarios with missed:" 8 "$(scale_expect_summary "$out" missed: planted)"
		assert_eq "miss passes both $arm controls" 2 "$(jq -s '[.[] | select(.scenario | startswith("control")) | select(.expect_ok)] | length' "$out"/*/*/score.json)"
		out="$TMP/mode-wrong-$arm"
		scale_modes "$arm" wrong "$out"
		assert_eq "wrong prints false-alarm: in all ten $arm scenarios" 10 "$(scale_expect_summary "$out" false-alarm: all)"
	done
}

scale_run_dir() { # scale_run_dir <dir> <scenario> <failures json> [transcript fixture] [extra json]: a run directory entry
	mkdir -p "$1/$2"
	jq -nc --arg scenario "$2" --argjson failures "$3" --argjson extra "${5:-null}" \
		'{scenario: $scenario, check_ok: true, expect_ok: ($failures | length == 0), expect_failures: $failures, setup_failed: false, runner_failed: false, usage: {read_before_first_write: true}} + ($extra // {})' >"$1/$2/score.json"
	if [[ -n "${4:-}" ]]; then cp "$HERE/transcripts/$4" "$1/$2/transcript.jsonl"; else : >"$1/$2/transcript.jsonl"; fi
}

scale_agg() { # scale_agg <run dir...>: the aggregate JSON
	bash "$SCALE/aggregate.sh" "$@"
}

test_scale_aggregate_rates() {
	echo "scale aggregate: rates"
	local caught="$TMP/agg-caught" missed="$TMP/agg-missed" json
	scale_small "$TMP/agg-scenarios"
	scale_run "$caught/out" "$TMP/agg-scenarios" "$HERE/runners/scale-solve.sh catch" t1-12-same-early-unprompted
	scale_run "$missed/out" "$TMP/agg-scenarios" "$HERE/runners/scale-solve.sh miss" t1-12-same-early-unprompted
	json="$(scale_agg "$caught"/out/* "$missed"/out/*)"
	assert_eq "a catch and a miss give catch rate 0.5 in the cell" "[2,1,0.5]" \
		"$(jq -c '.groups.cell[0] | [.runs, .catches, .catch_rate]' <<<"$json")"
	assert_eq "the row carries its dimensions" '["t1",12,"same","early","unprompted","unknown"]' \
		"$(jq -c '.rows[0] | [.type, .size, .wording, .position, .arm, .model]' <<<"$json")"
	assert_eq "a caught row says so and a missed row says so" "[true,false]" "$(jq -c '[.rows[].caught] | sort | reverse' <<<"$json")"
	assert_eq "the row fields are all present" '["arm","caught","cost_usd","false_alarm","incomplete","model","position","read_before_first_write","runner_failed","setup_failed","size","type","wording"]' \
		"$(jq -c '.rows[0] | del(.scenario, .scratch) | keys' <<<"$json")"
	assert_eq "the groups exist by every dimension" '["arm","cell","model","position","size","type","wording"]' "$(jq -c '.groups | keys' <<<"$json")"
	assert_eq "runs that wrote before reading are counted" '{"true":0,"false":2,"null":0}' "$(jq -c '.groups.arm[0].read_before_first_write' <<<"$json")"
	assert_eq "a stand-in with no cost has none" "null" "$(jq -c .total.cost_usd <<<"$json")"
	assert_eq "the total counts the runs" 2 "$(jq -c .total.runs <<<"$json")"
}

test_scale_aggregate_control_and_failures() {
	echo "scale aggregate: control and failed runs"
	local dir="$TMP/agg-fail" json
	scale_run_dir "$dir" t1-12-same-early-unprompted '[]'
	scale_run_dir "$dir" control-12-unprompted '["false-alarm: adr/0004 was superseded"]'
	scale_run_dir "$dir" t1-12-para-late-unprompted '["missed: x"]' "" '{"setup_failed": true}'
	scale_run_dir "$dir" t2-12-same-late-unprompted '["missed: x"]' "" '{"runner_failed": true}'
	json="$(scale_agg "$dir")"
	assert_eq "a control row has a null catch and a false alarm" '[null,true]' \
		"$(jq -c '.rows[] | select(.type == "control") | [.caught, .false_alarm]' <<<"$json")"
	assert_eq "all four runs have a row" 4 "$(jq -c '.rows | length' <<<"$json")"
	assert_eq "failed runs are flagged in their rows" '[true,true]' "$(jq -c '[(.rows[] | select(.scenario | startswith("t1-12-para")) | .setup_failed), (.rows[] | select(.scenario | startswith("t2")) | .runner_failed)]' <<<"$json")"
	assert_eq "failed runs stay out of the rates" '[1,1,1,0.5]' \
		"$(jq -c '.groups.arm[0] | [.catches, .catch_rate, .false_alarms, .false_alarm_rate]' <<<"$json")"
	assert_eq "a group without a planted run has a null catch rate" null \
		"$(jq -c '.groups.type[] | select(.key == "control") | .catch_rate' <<<"$json")"
}

scale_rule_json() { # scale_rule_json <u-large failures> <p-large failures> <u-small failures>: the rule block
	local dir
	dir="$(mktemp -d "$TMP/rule.XXXXXX")"
	scale_run_dir "$dir" t1-24-same-late-unprompted "$1"
	scale_run_dir "$dir" t1-24-same-late-prompted "$2"
	scale_run_dir "$dir" t1-12-same-early-unprompted "$3"
	scale_agg "$dir" | jq -c .rule
}

scale_rule_case() { # scale_rule_case <u-large failures> <p-large failures> <u-small failures>: the build_o2 verdict
	scale_rule_json "$@" | jq -c .build_o2
}

scale_rule_rates() { # scale_rule_rates <u-large failures> <p-large failures> <u-small failures>
	scale_rule_json "$@" | jq -c '[.unprompted_large, .prompted_large, .unprompted_small]'
}

test_scale_aggregate_rule() {
	echo "scale aggregate: decision rule"
	local miss='["missed: x"]'
	assert_eq "equal rates do not build" false "$(scale_rule_case '[]' '[]' '[]')"
	assert_eq "all missed, equal rates do not build" false "$(scale_rule_case "$miss" "$miss" "$miss")"
	assert_eq "unprompted large below prompted large builds" true "$(scale_rule_case "$miss" '[]' "$miss")"
	assert_eq "unprompted large below unprompted small only builds" true "$(scale_rule_case "$miss" "$miss" '[]')"
	assert_eq "the rule shows its three rates" '[0,1,1]' "$(scale_rule_rates "$miss" '[]' '[]')"
}

test_scale_aggregate_cost() {
	echo "scale aggregate: model and cost"
	local dir="$TMP/agg-cost" json
	scale_run_dir "$dir" t1-12-same-early-unprompted '[]' scale-model-cost.jsonl
	scale_run_dir "$dir" t1-12-same-early-prompted '[]' scale-model-cost-other.jsonl
	scale_run_dir "$dir" t2-12-same-late-unprompted '[]' read-only.jsonl
	json="$(scale_agg "$dir")"
	assert_eq "the model and the last result's cost come from the transcript" '["claude-sonnet-5-5",0.5]' \
		"$(jq -c '.rows[] | select(.scenario == "t1-12-same-early-unprompted") | [.model, .cost_usd]' <<<"$json")"
	assert_eq "a transcript without them has an unknown model and a null cost" '["unknown",null]' \
		"$(jq -c '.rows[] | select(.scenario == "t2-12-same-late-unprompted") | [.model, .cost_usd]' <<<"$json")"
	assert_eq "the total sums only known costs" "[0.75,3]" "$(jq -c '[.total.cost_usd, .total.runs]' <<<"$json")"
	assert_eq "a model group sums its own costs" "[\"claude-haiku-4-5\",0.25]" \
		"$(jq -c '.groups.model[] | select(.key == "claude-haiku-4-5") | [.key, .cost_usd]' <<<"$json")"
	assert_eq "the markdown view has a table per grouping" 1 \
		"$(scale_agg --markdown "$dir" | grep -c '^| claude-haiku-4-5 |')"
	assert_true "the markdown view states the rule" bash -c "bash '$SCALE/aggregate.sh' --markdown '$dir' | grep -q build_o2"
	bash "$SCALE/aggregate.sh" >/dev/null 2>&1
	assert_eq "no run directory exits 2" 2 "$?"
}

test_scale() {
	test_scale_generator
	test_scale_generator_usage
	test_scale_bundle
	test_scale_unprompted_mentions
	test_scale_unprompted_outcomes
	test_scale_unprompted_baseline
	test_scale_prompted
	test_scale_modes
	test_scale_aggregate_rates
	test_scale_aggregate_control_and_failures
	test_scale_aggregate_rule
	test_scale_aggregate_cost
}
