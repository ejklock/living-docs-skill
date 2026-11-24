#!/usr/bin/env bash
#
# scale.sh — tests for the scale scenarios: the generator, the bundles setup.sh builds, the
# unprompted and prompted scoring of expect.sh, the paraphrase vocabulary, and the aggregator. Sourced by run.sh, which
# provides TMP, HERE, EVALS, assert_eq, assert_true, run_eval and wanted. Scenarios are generated
# into $TMP; the scripted stand-in scale-solve.sh stands in for the agent.

SCALE="$EVALS/scale"
readonly SCALE_KIND=adr
readonly SCALE_NAMES="control-240 control-240-noise-late control-60 control-60-noise-early t1-240-para-early t1-240-same-late t1-60-para-late t1-60-same-early t2-240-para-late t2-240-same-early t2-60-para-early t2-60-same-late"

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
	assert_eq "twenty-four scenario directories with the planned names" "$(LC_ALL=C sort <<<"${expected%$'\n'}")" "$(find "$a" -mindepth 1 -maxdepth 1 -type d -exec basename {} \; | LC_ALL=C sort)"
	assert_eq "every scenario has its five files" 120 \
		"$(find "$a" -mindepth 2 -maxdepth 2 \( -name task.md -o -name setup.sh -o -name expect.sh -o -name manifest.tsv -o -name scenario.env \) | wc -l | tr -d ' ')"
	assert_eq "twelve prompted scenarios are written" 12 "$(scale_names "$a" prompted | wc -l | tr -d ' ')"
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
	for mention in "See $SCALE_KIND/0003." "See ${SCALE_KIND^^} 0003." "See ${SCALE_KIND^^}-3." "See 0003-keep-user-sessions-in-redis.md." "See ${SCALE_KIND^^}s 3." "See ${SCALE_KIND^^}s 0003 and 0005."; do
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

scale_conflicts() { # scale_conflicts <name> <content | NONE> [superseded number]: scores a scratch with that CONFLICTS.txt; sets OUTPUT
	local dir="$TMP/unp-scenarios" scratch new
	scratch="$(scale_case "$dir" "$1")"
	new="$(scale_add_adr "$scratch" "Nothing else.")"
	[[ -z "${3:-}" ]] || scale_ld "$scratch" write supersede "adr/$3" "adr/$new"
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

scale_modes() { # scale_modes <arm> <mode> <out>: runs the twelve scenarios of an arm through the stand-in
	local dir="$TMP/modes-scenarios" names
	[[ -d "$dir" ]] || scale_small "$dir"
	mapfile -t names < <(scale_names "$dir" "$1")
	scale_run "$3" "$dir" "$HERE/runners/scale-solve.sh $2" "${names[@]}"
}

scale_expect_summary() { # scale_expect_summary <out> <prefix> <planted|control|all>: scenarios passing or printing prefix
	jq -s -c --arg prefix "$2" --arg which "$3" '[.[] | select(($which == "all") or (($which == "control") == (.scenario | startswith("control"))))
		| select(.expect_failures | any(startswith($prefix)))] | length' "$1"/*/*/score.json
}

test_scale_prompted_lines() {
	echo "scale prompted scoring: one record per line"
	local plant=t1-24-same-late-prompted control=control-24-prompted listed
	scale_small "$TMP/unp-scenarios"
	scale_conflicts "$plant" "0022 Store uploads in S3 and serve them through the CDN"$'\n'
	assert_eq "a title line lists its leading number only" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	scale_conflicts "$control" "0022 Store uploads in S3 and serve them through the CDN"$'\n'
	assert_eq "the same line in a control is one false alarm" 1 "$(scale_count false-alarm:)"
	assert_true "the false alarm names 22 alone" grep -q '^false-alarm: 22 ' <<<"$OUTPUT"
	scale_conflicts "$control" "No conflicts (checked 240 ADRs)"$'\n'
	assert_eq "a sentence with a count lists nothing" "0 0" "$(scale_count false-alarm:) $(scale_count incomplete:)"
	scale_conflicts "$plant" "adr/0022 (2 clauses)"$'\n'
	assert_eq "a clause count is not a listed record" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	for listed in "22" "0022" "$SCALE_KIND/0022" "${SCALE_KIND^^} 0022" "- ${SCALE_KIND^^}-0022: title"; do
		scale_conflicts "$plant" "$listed"$'\n'
		assert_eq "the line '$listed' catches the plant" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	done
	scale_conflicts "$plant" $'0022\n0005\n'
	assert_eq "two lines list the plant and one other" "0 1" "$(scale_count missed:) $(scale_count false-alarm:)"
	assert_true "the other record is 5" grep -q '^false-alarm: 5 ' <<<"$OUTPUT"
}

test_scale_prompted_supersedes() {
	echo "scale prompted scoring: newly superseded records"
	scale_small "$TMP/unp-scenarios"
	scale_conflicts control-12-prompted "" 0004
	assert_eq "a prompted control superseding a record is a false alarm" 1 "$(scale_count false-alarm:)"
	scale_conflicts t1-12-same-early-prompted $'3\n' 0003
	assert_eq "listing and superseding the plant is a catch and clean" "0 0" "$(scale_count missed:) $(scale_count false-alarm:)"
	scale_conflicts t1-12-same-early-prompted $'3\n' 0004
	assert_eq "listing the plant and superseding another is a catch and a false alarm" "0 1" "$(scale_count missed:) $(scale_count false-alarm:)"
	scale_conflicts t1-12-same-early-prompted "" 0003
	assert_eq "superseding the plant without listing it stays a miss" "1 0" "$(scale_count missed:) $(scale_count false-alarm:)"
}

test_scale_paraphrase_vocabulary() {
	echo "scale paraphrase vocabulary"
	local dir="$TMP/para-vocab" line word
	scale_generate "$dir"
	line="$(grep -F 'Shared key-value store for login state' "$dir/t1-60-para-late-unprompted/manifest.tsv")"
	assert_true "the paraphrased plant line is found" test -n "$line"
	assert_true "the plant states one shared store of login state" grep -q -i 'one central key-value cluster' <<<"$line"
	for word in memory balancer load session sessions sticky node nodes app; do
		assert_true "the paraphrased plant does not say '$word'" test -z "$(grep -i -w -o "$word" <<<"$line")"
	done
}

test_scale_modes() {
	echo "scale scoring through run.sh"
	local arm out
	for arm in unprompted prompted; do
		out="$TMP/mode-catch-$arm"
		scale_modes "$arm" catch "$out"
		assert_eq "catch passes all twelve $arm scenarios" 12 "$(jq -s '[.[] | select(.expect_ok)] | length' "$out"/*/*/score.json)"
		out="$TMP/mode-miss-$arm"
		scale_modes "$arm" miss "$out"
		assert_eq "miss fails the eight planted $arm scenarios with missed:" 8 "$(scale_expect_summary "$out" missed: planted)"
		assert_eq "miss passes all four $arm controls" 4 "$(jq -s '[.[] | select(.scenario | startswith("control")) | select(.expect_ok)] | length' "$out"/*/*/score.json)"
		out="$TMP/mode-wrong-$arm"
		scale_modes "$arm" wrong "$out"
		assert_eq "wrong prints false-alarm: in all twelve $arm scenarios" 12 "$(scale_expect_summary "$out" false-alarm: all)"
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
	assert_eq "the row fields are all present" '["arm","caught","cost_usd","false_alarm","incomplete","model","outcome","over_linked","position","read_before_first_write","runner_failed","setup_failed","size","type","unscored","wording"]' \
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

test_scale_aggregate_unscored() {
	echo "scale aggregate: runs without a usable verdict"
	local dir="$TMP/agg-unscored" json
	scale_run_dir "$dir" t1-12-same-early-unprompted null
	scale_run_dir "$dir" t1-12-para-late-unprompted '["expect.sh exited 127 with no message"]'
	scale_run_dir "$dir" t2-12-same-late-unprompted '["incomplete: no CONFLICTS.txt", "missed: x"]'
	scale_run_dir "$dir" t2-12-para-early-unprompted '[]'
	scale_run_dir "$dir" control-12-unprompted '["something unknown"]'
	json="$(scale_agg "$dir")"
	assert_eq "a null verdict and an unknown line are unscored" \
		'{"control-12-unprompted":true,"t1-12-para-late-unprompted":true,"t1-12-same-early-unprompted":true,"t2-12-para-early-unprompted":false,"t2-12-same-late-unprompted":false}' \
		"$(jq -c '[.rows[] | {(.scenario): .unscored}] | add | to_entries | sort_by(.key) | from_entries' <<<"$json")"
	assert_eq "unscored runs are excluded and the other two are scored, one caught" '[5,2,3,2,1]' \
		"$(jq -c '.groups.arm[0] as $g | [.total.runs, $g.runs, $g.excluded, $g.planted, $g.catches]' <<<"$json")"
	assert_eq "an unscored row has no catch and still has its row" null "$(jq -c '.rows[] | select(.scenario == "t1-12-same-early-unprompted") | .caught' <<<"$json")"
}

test_scale_aggregate_per_model() {
	echo "scale aggregate: rule per model"
	local low="$TMP/agg-low" high="$TMP/agg-high" miss='["missed: x"]'
	scale_run_dir "$low" t1-24-same-late-unprompted "$miss" scale-model-cost.jsonl
	scale_run_dir "$low" t1-24-same-late-prompted '[]' scale-model-cost.jsonl
	scale_run_dir "$low" t1-12-same-early-unprompted '[]' scale-model-cost.jsonl
	scale_run_dir "$high" t1-24-same-late-unprompted '[]' scale-model-cost-other.jsonl
	scale_run_dir "$high" t1-24-same-late-prompted '[]' scale-model-cost-other.jsonl
	scale_run_dir "$high" t1-12-same-early-unprompted '[]' scale-model-cost-other.jsonl
	assert_eq "the model below builds and the model equal does not" '[true,false]' \
		"$(scale_agg "$low" "$high" | jq -c '.rule.per_model | [.["claude-sonnet-5-5"].build_o2, .["claude-haiku-4-5"].build_o2]')"
	assert_eq "each model has its three rates" '[0,1,1]' \
		"$(scale_agg "$low" "$high" | jq -c '.rule.per_model["claude-sonnet-5-5"] | [.unprompted_large, .prompted_large, .unprompted_small]')"
	assert_eq "the pooled rule is unchanged by the models" '[0.5,1,1,true]' \
		"$(scale_agg "$low" "$high" | jq -c '.rule | [.unprompted_large, .prompted_large, .unprompted_small, .build_o2]')"
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

test_scale_link_outcomes() {
	echo "scale unprompted scoring: superseded, named and over-linked"
	local dir="$TMP/unp-scenarios" name=t1-12-same-early-unprompted scratch new
	scale_small "$dir"
	scratch="$(scale_case "$dir" "$name")"
	new="$(scale_add_adr "$scratch" "Nothing else.")"
	scale_ld "$scratch" write supersede adr/0003 "adr/$new"
	scale_score "$dir" "$name" "$scratch"
	assert_eq "superseding the plant scores superseded and nothing over-linked" "1 0 0" \
		"$(scale_count 'outcome: superseded') $(scale_count 'outcome: named') $(scale_count over-linked:)"
	scratch="$(scale_case "$dir" "$name")"
	scale_add_adr "$scratch" "See adr/0003." >/dev/null
	scale_score "$dir" "$name" "$scratch"
	assert_eq "only naming the plant scores named, passes and nothing over-linked" "0 1 0 0" \
		"$(scale_count 'outcome: superseded') $(scale_count 'outcome: named') $(scale_count over-linked:) $RC"
	scratch="$(scale_case "$dir" "$name")"
	scale_add_adr "$scratch" "See adr/0003 and adr/0004." >/dev/null
	scale_score "$dir" "$name" "$scratch"
	assert_eq "naming the plant and another record in force scores named and over-linked" "0 1 1" \
		"$(scale_count 'outcome: superseded') $(scale_count 'outcome: named') $(scale_count over-linked:)"
	assert_true "the over-linked line carries the other record's number" grep -q '^over-linked: adr/0004 ' <<<"$OUTPUT"
	assert_eq "an over-link beside a catch is not a failure line" 0 "$(scale_count false-alarm:)"
	scratch="$(scale_case "$dir" "$name")"
	scale_add_adr "$scratch" "See adr/0004." >/dev/null
	scale_score "$dir" "$name" "$scratch"
	assert_eq "naming only another record is a miss with no outcome and one over-link" "1 0 0 1" \
		"$(scale_count missed:) $(scale_count 'outcome: superseded') $(scale_count 'outcome: named') $(scale_count over-linked:)"
	scratch="$(scale_case "$dir" control-12-unprompted)"
	scale_add_adr "$scratch" "See adr/0004." >/dev/null
	scale_score "$dir" control-12-unprompted "$scratch"
	assert_eq "a plain control reports an over-link without failing" "1 0 0" "$(scale_count over-linked:) $(scale_count false-alarm:) $RC"
}

scale_noise() { # scale_noise <name> <mention text> [superseded number] [CONFLICTS.txt content]: scores a noise control with a new ADR; sets OUTPUT and RC
	local dir="$TMP/unp-scenarios" scratch new
	scratch="$(scale_case "$dir" "$1")"
	new="$(scale_add_adr "$scratch" "$2")"
	[[ -z "${3:-}" ]] || scale_ld "$scratch" write supersede "adr/$3" "adr/$new"
	[[ -z "${4+set}" ]] || printf '%s' "$4" >"$scratch/CONFLICTS.txt"
	scale_score "$dir" "$1" "$scratch"
}

scale_other_in_force() { # scale_other_in_force <name> <number to avoid>: the padded number of the first Accepted ADR of the scenario's bundle other than that one
	local file
	scale_base "$TMP/unp-scenarios" "$1"
	for file in $(scale_adr_files "$TMP/base-$1"); do
		[[ "$(basename "$file" | cut -c1-4)" == "$2" ]] && continue
		grep -q '^status: Accepted$' "$file" && { basename "$file" | cut -c1-4; return; }
	done
}

test_scale_noise_controls() {
	echo "scale noise controls"
	local row name neighbor arm extra
	scale_small "$TMP/unp-scenarios"
	for row in "control-12-noise-early:0003" "control-24-noise-late:0022"; do
		for arm in unprompted prompted; do
			name="${row%%:*}-$arm"
			neighbor="${row##*:}"
			extra=()
			[[ "$arm" == prompted ]] && extra=("")
			scale_noise "$name" "Nothing else." "" ${extra[@]+"${extra[@]}"}
			assert_eq "$name passes when the new ADR links nothing" "0 0" "$(scale_count false-alarm:) $RC"
			scale_noise "$name" "Nothing else." 0004 ${extra[@]+"${extra[@]}"}
			assert_eq "$name fails on a supersede" "1 1" "$(scale_count false-alarm:) $RC"
			scale_noise "$name" "Nothing else." "$neighbor" ${extra[@]+"${extra[@]}"}
			assert_eq "$name fails when the noise ADR is superseded" "1 1" "$(scale_count false-alarm:) $RC"
			assert_true "$name names the superseded noise ADR" grep -q "^false-alarm: adr/$neighbor was superseded" <<<"$OUTPUT"
			scale_noise "$name" "See adr/$neighbor." "" ${extra[@]+"${extra[@]}"}
			assert_eq "$name passes and reports a link that leaves the noise ADR in force" "0 1 0" "$(scale_count false-alarm:) $(scale_count over-linked:) $RC"
			assert_true "$name names the over-linked record" grep -q "^over-linked: adr/$neighbor " <<<"$OUTPUT"
			scale_noise "$name" "See adr/$(scale_other_in_force "$name" "$neighbor")." "" ${extra[@]+"${extra[@]}"}
			assert_eq "$name passes and reports a link to any other record in force" "0 1 0" "$(scale_count false-alarm:) $(scale_count over-linked:) $RC"
		done
	done
}

test_scale_noise_vocabulary() {
	echo "scale noise vocabulary"
	local dir="$TMP/noise-vocab" name word shared position holders
	scale_generate "$dir"
	for name in control-60-noise-early-unprompted control-240-noise-late-unprompted; do
		shared=0
		position="$(grep '^SIZE=' "$dir/$name/scenario.env" | cut -d= -f2)"
		[[ "$name" == *-early-* ]] && position=3 || position=$((position - 2))
		for word in $(tr -c 'a-z' '\n' <"$dir/$name/task.md" | LC_ALL=C sort -u); do
			((${#word} >= 5)) || continue
			[[ "$(grep -c -i -w "$word" "$dir/$name/manifest.tsv")" == 1 ]] || continue
			holders="$(grep -i -w "$word" "$dir/$name/manifest.tsv" | cut -f1,2,3)"
			[[ "$holders" == "adr	$position	Reconcile the settlement ledger every night" ]] && shared=$((shared + 1))
		done
		assert_true "$name task shares two or more words found in exactly one record, the noise ADR at its position" test "$shared" -ge 2
		assert_eq "$name noise ADR is never superseded or deprecated" 0 \
			"$(awk -F'\t' -v n="$position" '($1 == "supersede" && ($2 == n || $3 == n)) || ($1 == "deprecate" && $2 == n)' "$dir/$name/manifest.tsv" | wc -l | tr -d ' ')"
		assert_eq "$name has no planted record" "PLANT=0" "$(grep '^PLANT=' "$dir/$name/scenario.env")"
		assert_eq "$name task asks for no supersede" 0 "$(grep -c -i supersede "$dir/$name/task.md")"
	done
	assert_eq "the noise manifest at 240 holds 240 ADRs" 240 "$(grep -c '^adr' "$dir/control-240-noise-late-unprompted/manifest.tsv")"
}

test_scale_output_root() {
	echo "scale output root"
	local data="$TMP/data-home"
	mkdir -p "$data"
	scale_small "$TMP/root-scenarios"
	(cd "$TMP" && XDG_DATA_HOME="$data" LIVING_DOCS_EVAL_SCENARIOS="$TMP/root-scenarios" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" control-12-unprompted >/dev/null 2>&1)
	assert_eq "the default output root is under the data home" 1 "$(find "$data/living-docs-evals" -name score.json 2>/dev/null | wc -l | tr -d ' ')"
	mkdir -p "$TMP/home-fallback"
	(cd "$TMP" && env -u XDG_DATA_HOME HOME="$TMP/home-fallback" LIVING_DOCS_EVAL_SCENARIOS="$TMP/root-scenarios" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" control-12-unprompted >/dev/null 2>&1)
	assert_eq "without a data home the root is under HOME/.local/share" 1 \
		"$(find "$TMP/home-fallback/.local/share/living-docs-evals" -name score.json 2>/dev/null | wc -l | tr -d ' ')"
	(cd "$TMP" && XDG_DATA_HOME="$data" LIVING_DOCS_EVAL_OUT="$TMP/chosen-out" LIVING_DOCS_EVAL_SCENARIOS="$TMP/root-scenarios" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" control-12-unprompted >/dev/null 2>&1)
	assert_eq "the existing variable overrides the root" 1 "$(find "$TMP/chosen-out" -name score.json | wc -l | tr -d ' ')"
}

test_scale_aggregate_links() {
	echo "scale aggregate: superseded, named and over-linked"
	local dir="$TMP/agg-links" json
	scale_run_dir "$dir" t1-12-same-early-unprompted '["outcome: superseded"]'
	scale_run_dir "$dir" t1-12-para-late-unprompted '["outcome: named", "over-linked: adr/0004 is named by a new record", "over-linked: adr/0007 is named by a new record"]'
	scale_run_dir "$dir" t2-12-same-late-unprompted '["missed: x"]'
	scale_run_dir "$dir" control-12-noise-early-unprompted '["over-linked: adr/0003 is named by a new record"]'
	json="$(scale_agg "$dir")"
	assert_eq "a superseding row has the outcome and no over-link" '["superseded",[]]' \
		"$(jq -c '.rows[] | select(.scenario == "t1-12-same-early-unprompted") | [.outcome, .over_linked]' <<<"$json")"
	assert_eq "a naming row has the outcome and its over-linked numbers" '["named",[4,7]]' \
		"$(jq -c '.rows[] | select(.scenario == "t1-12-para-late-unprompted") | [.outcome, .over_linked]' <<<"$json")"
	assert_eq "a missed row has no outcome" '[null,[]]' \
		"$(jq -c '.rows[] | select(.scenario == "t2-12-same-late-unprompted") | [.outcome, .over_linked]' <<<"$json")"
	assert_eq "a noise control row has its own type, no outcome and its over-link, not counted as a false alarm" '["noise-control",null,[3],false]' \
		"$(jq -c '.rows[] | select(.scenario | startswith("control-12-noise")) | [.type, .outcome, .over_linked, .false_alarm]' <<<"$json")"
	assert_eq "the arm group counts the plain scenarios' superseded, named and over-linked runs" '[1,1,1]' \
		"$(jq -c '.groups.arm[0] | [.superseded, .named, .over_linked]' <<<"$json")"
	assert_eq "caught stays caught for both outcomes" 2 "$(jq -c '.groups.arm[0].catches' <<<"$json")"
	assert_eq "every markdown table has the split columns" 7 "$(scale_agg --markdown "$dir" | grep -c '| superseded | named | over-linked |')"
}

test_scale_aggregate_noise_population() {
	echo "scale aggregate: noise controls stay out of the pooled rates"
	local plain="$TMP/agg-plain" mixed="$TMP/agg-mixed" before after dimension
	scale_run_dir "$plain" t1-12-same-early-unprompted '["outcome: superseded"]'
	scale_run_dir "$plain" control-12-unprompted '[]'
	scale_run_dir "$mixed" t1-12-same-early-unprompted '["outcome: superseded"]'
	scale_run_dir "$mixed" control-12-unprompted '[]'
	scale_run_dir "$mixed" control-12-noise-early-unprompted '["false-alarm: adr/0003 was superseded"]'
	before="$(scale_agg "$plain")"
	after="$(scale_agg "$mixed")"
	assert_eq "the control group's false alarm rate is unchanged by a noise control" "$(jq -c '.groups.type[] | select(.key == "control") | [.runs, .false_alarms, .false_alarm_rate]' <<<"$before")" \
		"$(jq -c '.groups.type[] | select(.key == "control") | [.runs, .false_alarms, .false_alarm_rate]' <<<"$after")"
	for dimension in size wording position arm model; do
		assert_eq "the $dimension groups are unchanged by a noise control" "$(jq -c ".groups.$dimension" <<<"$before")" "$(jq -c ".groups.$dimension" <<<"$after")"
	done
	assert_eq "the noise control has a type group of its own" '[1,1,0]' \
		"$(jq -c '.groups.type[] | select(.key == "noise-control") | [.runs, .false_alarms, .over_linked]' <<<"$after")"
	assert_eq "the total still counts every run" 3 "$(jq -c .total.runs <<<"$after")"
}

test_scale() {
	if [[ -n "${LIVING_DOCS_EVAL_ONLY:-}" ]]; then
		declare -F "$LIVING_DOCS_EVAL_ONLY" >/dev/null || { flunk "LIVING_DOCS_EVAL_ONLY" "no test function named $LIVING_DOCS_EVAL_ONLY"; return; }
		"$LIVING_DOCS_EVAL_ONLY"
		return
	fi
	test_scale_aggregate_noise_population
	test_scale_link_outcomes
	test_scale_noise_controls
	test_scale_noise_vocabulary
	test_scale_output_root
	test_scale_aggregate_links
	test_scale_generator
	test_scale_generator_usage
	test_scale_bundle
	test_scale_unprompted_mentions
	test_scale_unprompted_outcomes
	test_scale_unprompted_baseline
	test_scale_prompted
	test_scale_prompted_lines
	test_scale_prompted_supersedes
	test_scale_paraphrase_vocabulary
	test_scale_modes
	test_scale_aggregate_rates
	test_scale_aggregate_control_and_failures
	test_scale_aggregate_rule
	test_scale_aggregate_unscored
	test_scale_aggregate_per_model
	test_scale_aggregate_cost
}
