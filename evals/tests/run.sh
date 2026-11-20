#!/usr/bin/env bash
#
# run.sh — tests for the eval harness (run.sh, score.sh, score-transcript.sh), driven only by
# the scripted runners under runners/ and the fixture transcripts under transcripts/. No model
# is called and nothing leaves the machine. Scratch state lives under a directory made with
# mktemp and is removed on exit.
#
# usage: run.sh [usage|score|isolation|scenarios]   (no argument runs every group)
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
EVALS="$(cd "$HERE/.." && pwd)"
REPO="$(cd "$EVALS/.." && pwd)"
GROUP="${1:-all}"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/living-docs-evals-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
mkdir -p "$TMP/tmp"

resolve_living_docs() {
	local candidate
	if [[ -n "${LIVING_DOCS_BIN:-}" ]]; then
		candidate="$LIVING_DOCS_BIN"
	elif [[ -x "$REPO/target/release/living-docs" ]]; then
		candidate="$REPO/target/release/living-docs"
	else
		candidate="$(command -v living-docs || true)"
	fi
	[[ -n "$candidate" ]] || return 1
	printf '%s/%s' "$(cd "$(dirname "$candidate")" && pwd)" "$(basename "$candidate")"
}

LIVING_DOCS_BIN="$(resolve_living_docs)" || {
	echo "evals tests: no living-docs binary (set LIVING_DOCS_BIN or run make build)" >&2
	exit 2
}
export LIVING_DOCS_BIN

fail=0
cases=0

pass() {
	cases=$((cases + 1))
	printf '  ok    %s\n' "$1"
}

flunk() {
	cases=$((cases + 1))
	printf '  FAIL  %s: %s\n' "$1" "$2"
	fail=1
}

assert_eq() { # assert_eq <name> <expected> <actual>
	if [[ "$2" == "$3" ]]; then pass "$1"; else flunk "$1" "expected [$2] got [$3]"; fi
}

assert_true() { # assert_true <name> <command...>
	local name="$1"
	shift
	if "$@"; then pass "$name"; else flunk "$name" "condition false: $*"; fi
}

wanted() { [[ "$GROUP" == all || "$GROUP" == "$1" ]]; }

usage_field() { # usage_field <fixture> <jq path>
	bash "$EVALS/score-transcript.sh" "$HERE/transcripts/$1" | jq -c "$2"
}

run_eval() { # run_eval <out dir> <runner> [scenario...]; sets RC and STDOUT
	local out="$1" runner="$2"
	shift 2
	STDOUT="$(LIVING_DOCS_EVAL_OUT="$out" LIVING_DOCS_EVAL_RUNNER="$runner" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" "$@" 2>"$TMP/stderr")"
	RC=$?
}

score_json() { # score_json <out dir> <scenario>
	jq -c '.' "$1"/*/"$2"/score.json
}

docs_digest() {
	(cd "$REPO" && find docs -type f | LC_ALL=C sort | xargs sha256sum | sha256sum)
}

test_usage() {
	echo "usage block"
	assert_eq "three calls are counted" 3 "$(usage_field calls-three.jsonl .cli_calls)"
	assert_eq "calls are grouped by verb" '{"check":1,"read records":1,"write new":1}' \
		"$(usage_field calls-three.jsonl '.cli_calls_by_verb | to_entries | sort_by(.key) | from_entries')"
	assert_eq "a recognized transcript says so" true "$(usage_field calls-three.jsonl .transcript_recognized)"
	assert_eq "no hand edit in a cli-only session" 0 "$(usage_field calls-three.jsonl .owned_field_hand_edits)"
	assert_eq "an unknown subcommand is counted" 1 "$(usage_field unknown-subcommand.jsonl .unknown_subcommand)"
	assert_eq "an unknown subcommand is a cli error" 1 "$(usage_field unknown-subcommand.jsonl .cli_errors)"
	assert_eq "a help flag is a probe" 1 "$(usage_field help-probe.jsonl .help_probes)"
	assert_eq "a bare number reference is counted" 1 "$(usage_field bare-number-ref.jsonl .bare_number_refs)"
	assert_eq "a typed reference is not counted" 0 "$(usage_field typed-ref.jsonl .bare_number_refs)"
	assert_eq "a read before the first write" true "$(usage_field read-before-write.jsonl .read_before_first_write)"
	assert_eq "a read after the first write" false "$(usage_field write-before-read.jsonl .read_before_first_write)"
	assert_eq "an edit of a frontmatter line is a hand edit" 1 "$(usage_field edit-frontmatter.jsonl .owned_field_hand_edits)"
	assert_eq "an edit of body text is not" 0 "$(usage_field edit-body.jsonl .owned_field_hand_edits)"
	assert_eq "a write of an index is a hand edit" 1 "$(usage_field write-index.jsonl .owned_field_hand_edits)"
	assert_eq "a call inside a compound command is counted" 1 "$(usage_field compound-command.jsonl .cli_calls)"
	assert_eq "a non-zero exit of a cli call is an error" 1 "$(usage_field failed-call.jsonl .cli_errors)"
	assert_eq "a failing other command is not a cli error" 0 "$(usage_field failed-other-command.jsonl .cli_errors)"
	assert_eq "text inside a heredoc is not a call" 0 "$(usage_field heredoc-text.jsonl .cli_calls)"
	assert_eq "a malformed line never stops the count" 3 "$(usage_field malformed-lines.jsonl .cli_calls)"
	assert_eq "a string input is not a command" 0 "$(usage_field malformed-lines.jsonl .bare_number_refs)"
	assert_eq "a malformed transcript is still recognized" true "$(usage_field malformed-lines.jsonl .transcript_recognized)"
	bash "$EVALS/score-transcript.sh" "$HERE/transcripts/malformed-lines.jsonl" >/dev/null
	assert_eq "a malformed line exits zero" 0 "$?"
	assert_eq "exit code text in a successful output is not an error" 0 "$(usage_field exit-text-success.jsonl .cli_errors)"
	assert_eq "a result that begins with an exit code is an error" 1 "$(usage_field exit-prefix.jsonl .cli_errors)"
	assert_eq "a result flagged as an error is an error" 1 "$(usage_field error-flag.jsonl .cli_errors)"
	assert_eq "a backslash-newline keeps the verb" '{"write set":1}' "$(usage_field continuation.jsonl .cli_calls_by_verb)"
	assert_eq "a backslash-newline keeps the reference" 1 "$(usage_field continuation.jsonl .bare_number_refs)"
	assert_eq "reads and no write" true "$(usage_field read-only.jsonl .read_before_first_write)"
	assert_eq "neither a read nor a write" null "$(usage_field no-activity.jsonl .read_before_first_write)"
	assert_eq "a write and no read" false "$(usage_field write-only.jsonl .read_before_first_write)"
	assert_eq "a body edit that looks like a key is not a hand edit" 0 "$(usage_field edit-fence-type.jsonl .owned_field_hand_edits)"
	assert_eq "an edit holding the opening delimiter is a hand edit" 1 "$(usage_field edit-opening-delimiter.jsonl .owned_field_hand_edits)"
	assert_eq "a horizontal rule in a body edit is not a hand edit" 0 "$(usage_field edit-horizontal-rule.jsonl .owned_field_hand_edits)"
	assert_eq "a rule followed later by a fenced key is not a hand edit" 0 "$(usage_field edit-rule-then-fenced-key.jsonl .owned_field_hand_edits)"
	assert_eq "a rule followed by a blank line and a key is not a hand edit" 0 "$(usage_field edit-rule-then-key-after-blank.jsonl .owned_field_hand_edits)"
	assert_eq "two weak key lines are a hand edit" 1 "$(usage_field edit-two-weak-keys.jsonl .owned_field_hand_edits)"
	assert_eq "one weak key line is not" 0 "$(usage_field edit-one-weak-key.jsonl .owned_field_hand_edits)"
	assert_eq "unknown-subcommand text in a successful output is not counted" 0 "$(usage_field unknown-text-success.jsonl .unknown_subcommand)"
	assert_eq "an edit of the leading block is a hand edit" 1 "$(usage_field edit-leading-block.jsonl .owned_field_hand_edits)"
	assert_eq "a write of a whole record is a hand edit" 1 "$(usage_field write-record.jsonl .owned_field_hand_edits)"
	assert_eq "an edit of a callout is a hand edit" 1 "$(usage_field edit-callout.jsonl .owned_field_hand_edits)"
	assert_eq "a multi-edit of a callout is a hand edit" 1 "$(usage_field multiedit-callout.jsonl .owned_field_hand_edits)"
	local fixture
	for fixture in empty.jsonl not-json.txt other-shape.json; do
		assert_eq "$fixture is not recognized" false "$(usage_field "$fixture" .transcript_recognized)"
		assert_eq "$fixture has null counts" null "$(usage_field "$fixture" .cli_calls)"
	done
	bash "$EVALS/score-transcript.sh" "$HERE/transcripts/empty.jsonl" >/dev/null
	assert_eq "an unrecognized transcript exits zero" 0 "$?"
}

test_score() {
	echo "final-tree score"
	local out="$TMP/score-out"
	run_eval "$out" "$HERE/runners/solve.sh" record-decision
	assert_eq "a solved task passes the check" true "$(score_json "$out" record-decision | jq -c .check_ok)"
	assert_eq "a solved task meets its expectations" true "$(score_json "$out" record-decision | jq -c .expect_ok)"
	assert_eq "a solved run exits zero" 0 "$RC"
	assert_eq "a solved task used the cli" true "$(score_json "$out" record-decision | jq -c '.usage.cli_calls > 0')"

	out="$TMP/score-hand"
	run_eval "$out" "$HERE/runners/hand-edit-status.sh" supersede-adr
	assert_eq "a hand-set status fails the check" false "$(score_json "$out" supersede-adr | jq -c .check_ok)"
	assert_eq "the violation messages are reported" true \
		"$(score_json "$out" supersede-adr | jq -c '(.violations | length > 0) and all(.violations[]; (.message | length) > 0)')"
	assert_eq "a failing run exits one" 1 "$RC"

	out="$TMP/score-noop"
	run_eval "$out" "$HERE/runners/noop.sh" record-decision
	assert_eq "a runner that does nothing fails its expectation" false "$(score_json "$out" record-decision | jq -c .expect_ok)"
	assert_eq "the failed expectation is named" true \
		"$(score_json "$out" record-decision | jq -c '(.expect_failures | length > 0) and all(.expect_failures[]; length > 0)')"

	out="$TMP/score-noop-all"
	run_eval "$out" "$HERE/runners/noop.sh"
	assert_eq "a runner that does nothing fails the run" 1 "$RC"
	assert_eq "a runner that does nothing passes only the two scenarios that need no change" \
		'["answer-from-existing-decision","revive-superseded-adr"]' \
		"$(jq -s -c '[.[] | select(.check_ok and .expect_ok) | .scenario] | sort' "$out"/*/*/score.json)"

	out="$TMP/score-callout"
	run_eval "$out" "$HERE/runners/hand-edit-callout.sh" legacy-callout-wording
	assert_eq "a hand-edited callout is counted" true "$(score_json "$out" legacy-callout-wording | jq -c '.usage.owned_field_hand_edits > 0')"
	assert_eq "a hand-edited callout fails the expectation by name" true \
		"$(score_json "$out" legacy-callout-wording | jq -c '.expect_failures | any(test("by hand"))')"
	out="$TMP/score-callout-index"
	run_eval "$out" "$HERE/runners/hand-edit-callout-then-index.sh" legacy-callout-wording
	assert_eq "regenerating the index does not excuse a hand edit" true \
		"$(score_json "$out" legacy-callout-wording | jq -c '(.check_ok) and (.expect_failures | any(test("by hand")))')"
	assert_eq "a hand edit fails the run" 1 "$RC"

	out="$TMP/score-silent"
	run_eval "$out" "$HERE/runners/repair-without-transcript.sh" legacy-callout-wording
	assert_eq "a repaired bundle with no transcript still has a green tree" true "$(score_json "$out" legacy-callout-wording | jq -c .check_ok)"
	assert_eq "hand edits that cannot be verified fail the scenario" true \
		"$(score_json "$out" legacy-callout-wording | jq -c '(.expect_ok | not) and (.expect_failures | any(test("not verifiable")))')"
	out="$TMP/score-solved-legacy"
	run_eval "$out" "$HERE/runners/solve.sh" legacy-callout-wording
	assert_eq "the scripted solver with its transcript passes the legacy scenario" 0 "$RC"

	out="$TMP/score-unreadable"
	LIVING_DOCS_EVAL_KEEP=1 LIVING_DOCS_EVAL_OUT="$out" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/solve.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" legacy-callout-wording >/dev/null 2>&1
	local scratch unreadable
	scratch="$(jq -r .scratch "$out"/*/legacy-callout-wording/score.json)"
	for unreadable in "$TMP" "$TMP/no-such-transcript"; do
		assert_eq "a transcript given as $(basename "$unreadable") keeps the tree score and fails the verification" true \
			"$(bash "$EVALS/score.sh" "$scratch" "$unreadable" "$EVALS/scenarios/legacy-callout-wording/expect.sh" |
				jq -c '.check_ok and (.usage.transcript_recognized | not) and (.expect_ok | not) and (.expect_failures | any(test("not verifiable")))')"
	done
	mkdir -p "$TMP/failing-scorer"
	cp "$EVALS/score.sh" "$TMP/failing-scorer/score.sh"
	printf '%s\n' '#!/usr/bin/env bash' 'exit 1' >"$TMP/failing-scorer/score-transcript.sh"
	assert_eq "a transcript scorer that fails keeps the tree score and fails the verification" true \
		"$(bash "$TMP/failing-scorer/score.sh" "$scratch" "$TMP/no-such-transcript" "$EVALS/scenarios/legacy-callout-wording/expect.sh" |
			jq -c '.check_ok and (.usage.transcript_recognized | not) and (.expect_ok | not) and (.expect_failures | any(test("not verifiable")))')"
	rm -rf "$scratch"

	out="$TMP/score-malformed"
	run_eval "$out" "$HERE/runners/solve-then-malformed.sh" record-decision
	assert_eq "malformed transcript lines keep the tree score" 0 "$RC"
	assert_eq "the table still shows check and expectations passed" 1 "$(grep -c -E '^record-decision +pass +ok +ok' <<<"$STDOUT")"

	out="$TMP/score-fail"
	run_eval "$out" "$HERE/runners/fail.sh" record-decision supersede-adr
	assert_eq "a failing runner is marked" true "$(score_json "$out" record-decision | jq -c .runner_failed)"
	assert_eq "the run continues with the next scenario" true "$(score_json "$out" supersede-adr | jq -c .runner_failed)"
	assert_eq "a failing runner's transcript is scored" true "$(score_json "$out" supersede-adr | jq -c .usage.transcript_recognized)"
	assert_eq "a failing runner's tree is scored" true "$(score_json "$out" supersede-adr | jq -c '(.expect_failures | length) > 0')"
	assert_eq "a failing runner fails the run" 1 "$RC"
	assert_eq "the table has a row per scenario" 2 "$(grep -c -E '^(record-decision|supersede-adr) ' <<<"$STDOUT")"
}

test_scenarios() {
	echo "scenarios"
	local out="$TMP/scenarios-out" count name
	run_eval "$out" "$HERE/runners/solve.sh"
	assert_eq "the scripted solver passes every scenario" 0 "$RC"
	count="$(find "$EVALS/scenarios" -mindepth 1 -maxdepth 1 -type d | wc -l | tr -d ' ')"
	assert_eq "eight scenarios exist" 8 "$count"
	assert_eq "the table shows a row per scenario" 8 "$(grep -c -E '^[a-z][a-z-]+ +pass' <<<"$STDOUT")"
	assert_eq "every scenario passes check and expectations" 8 \
		"$(jq -s '[.[] | select(.check_ok and .expect_ok)] | length' "$out"/*/*/score.json)"
	for name in "$EVALS"/scenarios/*/; do
		name="$(basename "$name")"
		assert_true "$name has a task, a setup and an expectation" \
			test -s "$EVALS/scenarios/$name/task.md" -a -f "$EVALS/scenarios/$name/setup.sh" -a -f "$EVALS/scenarios/$name/expect.sh"
	done
}

make_probe_scenarios() {
	local root="$1"
	mkdir -p "$root/probe"
	printf '%s\n' 'say $(touch PWNED); `touch PWNED` "q" '"'"'a'"'"' & | > < ; \ *' >"$root/probe/task.md"
	printf '%s\n' ':' >"$root/probe/setup.sh"
	printf '%s\n' 'exit 0' >"$root/probe/expect.sh"
}

make_odd_name_scenarios() {
	local root="$1" name
	for name in 'a b' '-dash' $'new\nline'; do
		mkdir -p "$root/$name"
		printf '%s\n' 'task' >"$root/$name/task.md"
		printf '%s\n' ':' >"$root/$name/setup.sh"
		printf '%s\n' 'exit 0' >"$root/$name/expect.sh"
	done
}

test_isolation() {
	echo "isolation"
	local before after out name
	before="$(docs_digest)"
	run_eval "$TMP/iso-out" "$HERE/runners/solve.sh" record-decision
	after="$(docs_digest)"
	assert_eq "the repository docs are unchanged by a run" "$before" "$after"

	local a="$TMP/iso-a" b="$TMP/iso-b"
	(LIVING_DOCS_EVAL_KEEP=1 LIVING_DOCS_EVAL_OUT="$a" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/solve.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" record-decision >/dev/null 2>&1) &
	(LIVING_DOCS_EVAL_KEEP=1 LIVING_DOCS_EVAL_OUT="$b" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/solve.sh" TMPDIR="$TMP/tmp" \
		bash "$EVALS/run.sh" record-decision >/dev/null 2>&1) &
	wait
	local scratch_a scratch_b
	scratch_a="$(jq -r .scratch "$a"/*/record-decision/score.json)"
	scratch_b="$(jq -r .scratch "$b"/*/record-decision/score.json)"
	assert_true "two simultaneous runs use two scratch directories" test -n "$scratch_a" -a "$scratch_a" != "$scratch_b"
	assert_true "a kept scratch directory still exists" test -d "$scratch_a" -a -d "$scratch_b"
	rm -rf "$scratch_a" "$scratch_b"

	for name in '../x' 'a/b' '..' '/etc' '../scenarios/record-decision'; do
		out="$TMP/iso-refused"
		run_eval "$out" "$HERE/runners/noop.sh" "$name"
		assert_eq "the scenario name '$name' is refused" 2 "$RC"
		assert_true "a refused name leaves no output" test ! -e "$out"
	done

	local root="$TMP/probe-scenarios"
	make_probe_scenarios "$root"
	out="$TMP/iso-probe"
	STDOUT="$(LIVING_DOCS_EVAL_SCENARIOS="$root" LIVING_DOCS_EVAL_OUT="$out" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/capture-task.sh" \
		LIVING_DOCS_EVAL_PROBE="$TMP/probe-task.txt" TMPDIR="$TMP/tmp" bash "$EVALS/run.sh" probe 2>"$TMP/stderr")"
	assert_true "a task with shell metacharacters reaches the runner unchanged" cmp -s "$root/probe/task.md" "$TMP/probe-task.txt"

	make_odd_name_scenarios "$TMP/odd-scenarios"
	for name in 'a b' '-dash' $'new\nline'; do
		out="$TMP/iso-odd"
		STDOUT="$(LIVING_DOCS_EVAL_SCENARIOS="$TMP/odd-scenarios" LIVING_DOCS_EVAL_OUT="$out" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" \
			TMPDIR="$TMP/tmp" bash "$EVALS/run.sh" "$name" 2>"$TMP/stderr")"
		assert_eq "the scenario name '$name' is refused although its directory exists" 2 "$?"
		assert_true "a refused odd name leaves no output" test ! -e "$out"
	done

	mkdir -p "$TMP/start dir"
	cp "$HERE/runners/capture-task.sh" "$TMP/start dir/runner.sh"
	(cd "$TMP/start dir" && LIVING_DOCS_EVAL_SCENARIOS="$root" LIVING_DOCS_EVAL_OUT="$TMP/iso-space" LIVING_DOCS_EVAL_RUNNER=./runner.sh \
		LIVING_DOCS_EVAL_PROBE="$TMP/probe-space.txt" TMPDIR="$TMP/tmp" bash "$EVALS/run.sh" probe >/dev/null 2>&1)
	assert_true "a relative runner runs from a start directory with a space" cmp -s "$root/probe/task.md" "$TMP/probe-space.txt"

	local quoted_out="$TMP/o\"ut" quoted_tmp="$TMP/t\"mp"
	mkdir -p "$quoted_tmp"
	LIVING_DOCS_EVAL_KEEP=1 LIVING_DOCS_EVAL_OUT="$quoted_out" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" TMPDIR="$quoted_tmp" \
		bash "$EVALS/run.sh" record-decision >/dev/null 2>&1
	assert_eq "a path with a double quote keeps the result valid JSON" "$quoted_tmp" \
		"$(dirname "$(jq -r .scratch "$quoted_out"/*/record-decision/score.json)")"
	rm -rf "$quoted_tmp"

	assert_eq "scratch directories are removed at exit" 0 "$(find "$TMP/tmp" -mindepth 1 -maxdepth 1 | wc -l | tr -d ' ')"
	LIVING_DOCS_EVAL_KEEP=1 LIVING_DOCS_EVAL_SCENARIOS="$root" LIVING_DOCS_EVAL_OUT="$out" LIVING_DOCS_EVAL_RUNNER="$HERE/runners/noop.sh" \
		TMPDIR="$TMP/tmp" bash "$EVALS/run.sh" probe >/dev/null 2>&1
	assert_eq "scratch directories are kept when asked" true "$(find "$TMP/tmp" -mindepth 1 -maxdepth 1 | grep -q . && echo true || echo false)"
}

wanted usage && test_usage
wanted score && test_score
wanted scenarios && test_scenarios
wanted isolation && test_isolation

printf '%s cases\n' "$cases"
((fail == 0))
