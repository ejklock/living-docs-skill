#!/usr/bin/env bash
#
# run.sh — drive an agent through documentation tasks in scratch bundles and score each result.
#
# usage: run.sh [SCENARIO...]    (no name runs every directory under evals/scenarios)
#
# For each scenario: make a scratch git repository under $TMPDIR, put the binary under test
# first on PATH, run scenarios/NAME/setup.sh in it, pipe scenarios/NAME/task.md to the runner
# command with the scratch repository as working directory, save the runner's stdout as the
# transcript, and score the final tree with score.sh. Ends with a table; exit 0 when every
# scenario has check_ok and expect_ok, 1 otherwise, 2 on bad usage.
#
# environment:
#   LIVING_DOCS_BIN             binary under test (default: target/release/living-docs of this checkout)
#   LIVING_DOCS_EVAL_RUNNER     runner command line (default: claude -p, stream-json, acceptEdits)
#   LIVING_DOCS_EVAL_OUT        output directory (default: ${XDG_DATA_HOME:-$HOME/.local/share}/living-docs-evals, outside any worktree); results go to OUT/RUN_ID/NAME/
#   LIVING_DOCS_EVAL_TIMEOUT    seconds allowed per runner call (default: 900; needs timeout(1) to apply)
#   LIVING_DOCS_EVAL_KEEP       1 keeps the scratch directories
#   LIVING_DOCS_EVAL_SCENARIOS  scenarios directory (default: evals/scenarios)
#
# The runner receives LIVING_DOCS_EVAL_SCENARIO (the scenario name) in its environment. Needs bash and jq.

set -uo pipefail
export LC_ALL=C

readonly DEFAULT_RUNNER='claude -p --output-format stream-json --verbose --permission-mode acceptEdits'
readonly NAME_PATTERN='^[A-Za-z0-9][A-Za-z0-9._-]*$'

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/.." && pwd)"
START_DIR="$PWD"
SCENARIOS="${LIVING_DOCS_EVAL_SCENARIOS:-$HERE/scenarios}"
RUNNER="${LIVING_DOCS_EVAL_RUNNER:-$DEFAULT_RUNNER}"
TIMEOUT="${LIVING_DOCS_EVAL_TIMEOUT:-900}"
KEEP="${LIVING_DOCS_EVAL_KEEP:-0}"
OUT="${LIVING_DOCS_EVAL_OUT:-${XDG_DATA_HOME:-${HOME:-/tmp}/.local/share}/living-docs-evals}"
RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)-$$"
SCRATCH=""
CLEANUP=()
NAMES=()
ALL_PASS=1

die() {
	printf 'run: %s\n' "$*" >&2
	exit 2
}

absolute() {
	case "$1" in
	/*) printf '%s' "$1" ;;
	*) printf '%s/%s' "$START_DIR" "$1" ;;
	esac
}

cleanup() {
	local path
	for path in ${CLEANUP[@]+"${CLEANUP[@]}"}; do
		rm -rf "$path"
	done
}
trap cleanup EXIT

resolve_binary() {
	local bin="${LIVING_DOCS_BIN:-$ROOT/target/release/living-docs}"
	bin="$(absolute "$bin")"
	[[ -x "$bin" ]] || die "living-docs binary not found or not executable: $bin"
	printf '%s' "$bin"
}

absolute_runner() {
	local first="${RUNNER%%[[:space:]]*}"
	if [[ "$first" == */* && "$first" != /* && -e "$START_DIR/$first" ]]; then
		RUNNER="$(printf '%q' "$START_DIR/$first")${RUNNER#"$first"}"
	fi
}

collect_names() {
	local name dir
	if (($# > 0)); then
		NAMES=("$@")
	else
		for dir in "$SCENARIOS"/*/; do
			[[ -d "$dir" ]] && NAMES+=("$(basename "$dir")")
		done
	fi
	((${#NAMES[@]} > 0)) || die "no scenario to run under $SCENARIOS"
	for name in "${NAMES[@]}"; do
		[[ "$name" =~ $NAME_PATTERN && "$name" != *..* ]] || die "refusing scenario name: $name"
		[[ -s "$SCENARIOS/$name/task.md" && -f "$SCENARIOS/$name/setup.sh" && -f "$SCENARIOS/$name/expect.sh" ]] ||
			die "scenario $name needs task.md, setup.sh and expect.sh under $SCENARIOS/$name"
	done
}

make_scratch() {
	SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/living-docs-eval.XXXXXX")" || die "cannot make a scratch directory"
	[[ "$KEEP" == 1 ]] || CLEANUP+=("$SCRATCH")
	git -C "$SCRATCH" init -q
	git -C "$SCRATCH" config user.name "Eval Author"
	git -C "$SCRATCH" config user.email "eval@example.com"
}

run_runner() { # run_runner <scratch> <task> <transcript> <stderr>
	local limit=()
	if command -v timeout >/dev/null 2>&1 && [[ "$TIMEOUT" =~ ^[1-9][0-9]*$ ]]; then
		limit=(timeout "$TIMEOUT")
	fi
	(cd "$1" && ${limit[@]+"${limit[@]}"} bash -c "$RUNNER" <"$2" >"$3" 2>"$4")
}

record() { # record <result file> <scenario> <setup_failed> <runner_exit> <score json>
	local kept=""
	[[ "$KEEP" == 1 ]] && kept="$6"
	jq -c --arg scenario "$2" --argjson setup_failed "$3" --argjson runner_exit "$4" --arg scratch "$kept" \
		'. + {scenario: $scenario, setup_failed: $setup_failed, runner_exit: $runner_exit, runner_failed: ($runner_exit != 0), scratch: (if $scratch == "" then null else $scratch end)}' \
		<<<"$5" >"$1"
}

run_one() { # run_one <name>
	local name="$1" scenario="$SCENARIOS/$1" dir scratch score
	dir="$OUT/$RUN_ID/$name"
	mkdir -p "$dir"
	make_scratch
	scratch="$SCRATCH"
	export LIVING_DOCS_EVAL_SCENARIO="$name"
	if ! (cd "$scratch" && bash "$scenario/setup.sh") >"$dir/setup.log" 2>&1; then
		record "$dir/score.json" "$name" true 0 '{"check_ok":false,"expect_ok":false,"fmt_pending":[],"usage":{}}' "$scratch"
		return
	fi
	local rc=0
	run_runner "$scratch" "$scenario/task.md" "$dir/transcript.jsonl" "$dir/runner.stderr" || rc=$?
	score="$(bash "$HERE/score.sh" "$scratch" "$dir/transcript.jsonl" "$scenario/expect.sh")" ||
		score='{"check_ok":false,"expect_ok":false,"fmt_pending":[],"usage":{}}'
	record "$dir/score.json" "$name" false "$rc" "$score" "$scratch"
}

print_table() {
	local name file row pass=0
	printf '%-34s %-6s %-6s %-6s %-4s %-6s %-6s %-6s %s\n' SCENARIO RESULT CHECK EXPECT FMT CALLS ERRORS HANDED RUNNER
	for name in "${NAMES[@]}"; do
		file="$OUT/$RUN_ID/$name/score.json"
		row="$(jq -r '[.scenario, (if .check_ok and .expect_ok and (.setup_failed | not) then "pass" else "FAIL" end),
			(if .check_ok then "ok" else "red" end), (if .expect_ok then "ok" else "red" end),
			(.fmt_pending | length), (.usage.cli_calls // "-"), (.usage.cli_errors // "-"), (.usage.owned_field_hand_edits // "-"),
			(if .setup_failed then "setup-failed" elif .runner_failed then "failed(\(.runner_exit))" else "ok" end)] | @tsv' "$file")"
		IFS=$'\t' read -r -a cells <<<"$row"
		printf '%-34s %-6s %-6s %-6s %-4s %-6s %-6s %-6s %s\n' "${cells[@]}"
		[[ "${cells[1]}" == pass ]] && pass=$((pass + 1)) || ALL_PASS=0
	done
	printf '%s of %s scenarios pass; results in %s/%s\n' "$pass" "${#NAMES[@]}" "$OUT" "$RUN_ID"
}

main() {
	command -v jq >/dev/null 2>&1 || die "jq not found on PATH"
	collect_names "$@"
	LIVING_DOCS_BIN="$(resolve_binary)"
	export LIVING_DOCS_BIN
	OUT="$(absolute "$OUT")"
	absolute_runner
	local bindir name
	bindir="$(mktemp -d "${TMPDIR:-/tmp}/living-docs-eval-bin.XXXXXX")" || die "cannot make a scratch directory"
	CLEANUP+=("$bindir")
	ln -s "$LIVING_DOCS_BIN" "$bindir/living-docs"
	export PATH="$bindir:$PATH"
	for name in "${NAMES[@]}"; do
		run_one "$name"
	done
	print_table
	((ALL_PASS == 1))
}

main "$@"
