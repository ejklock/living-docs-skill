#!/usr/bin/env bash
#
# aggregate.sh — turn the run directories of the scale scenarios into catch and false-alarm rates.
#
# usage: aggregate.sh [--markdown] RUN_DIR...
#
# A RUN_DIR is one run.sh result directory (OUT/RUN_ID) holding NAME/score.json and
# NAME/transcript.jsonl per scenario run. Prints one JSON object:
#   rows    one per scenario run: type, size, wording, position, arm, model (the transcript's init
#           line, else "unknown"), caught (null for a control), outcome ("superseded" or "named"
#           for a caught unprompted plant, else null), over_linked (the numbers of the records in
#           force that a new record names, other than the plant), surviving_clause ("kept" or
#           "lost" for a partial reversal: whether a record in force still holds the clause the
#           reversal keeps, else null), false_alarm, incomplete,
#           read_before_first_write, cost_usd (the last result line, else null), setup_failed,
#           runner_failed, unscored
#   groups  by type, size, wording, position, arm, model and the cell type/size/wording/arm/model
#           (a noise control has the type noise-control, an exception run x1 and an exception
#           control exception-control; these appear only in the type and cell groups and stay out
#           of the rule, so every other group and the rule keep the population of the plain
#           scenarios):
#           runs, excluded, planted, catches, catch_rate (null with no planted run) and catch_ci
#           (its 95% Wilson score interval, [low, high]), superseded and
#           named (the planted runs of each outcome; catches counts both), over_linked (runs with
#           any), clause_lost (runs whose kept clause left force), false_alarms, false_alarm_ci, false_alarm_rate, read_before_first_write counts and the summed
#           known cost
#   total   runs, excluded and the summed known cost
#   rule    the catch rate of the unprompted arm at the large size, of the prompted arm at the large
#           size and of the unprompted arm at the small size, and build_o2: true exactly when the
#           first is below either of the others; per_model holds the same three rates and build_o2
#           for each model
# A run whose setup or runner failed, or whose expect_failures is null or holds a line starting with
# none of missed:, false-alarm:, incomplete:, outcome:, over-linked:, surviving-clause: (unscored), has a row and is excluded from every rate. --markdown prints
# the same as tables. Needs bash and jq. Exit: 0 = printed, 2 = bad usage.

set -uo pipefail
export LC_ALL=C

readonly NAME_PATTERN='^(?<type>t1|t2|x1|control)-(?<size>[0-9]+)(-(?<wording>[a-z]+)-(?<position>[a-z]+))?-(?<arm>unprompted|prompted)$'

readonly TRANSCRIPT_PROGRAM='
split("\n") | map(try fromjson catch empty) | map(select(type == "object")) |
{model: ([.[] | select(.type == "system" and .subtype == "init" and ((.model | type) == "string")) | .model] | first // "unknown"),
 cost_usd: ([.[] | select(.type == "result" and ((.total_cost_usd | type) == "number")) | .total_cost_usd] | last)}'

readonly ROW_PROGRAM='
(.scenario | capture($pattern)) as $n | .expect_failures as $raw |
(($raw | type) != "array" or ($raw | any(type != "string" or (startswith("missed:") or startswith("false-alarm:") or startswith("incomplete:") or startswith("outcome:") or startswith("over-linked:") or startswith("surviving-clause:") | not)))) as $unscored |
(if ($raw | type) == "array" then [$raw[] | select(type == "string")] else [] end) as $lines |
{scenario, type: (if $n.wording == "noise" then "noise-control" elif ($n.type == "control" and (($n.wording // "") | startswith("x"))) then "exception-control" else $n.type end), size: ($n.size | tonumber), wording: ($n.wording // "none"), position: ($n.position // "none"),
 arm: $n.arm, model: $t.model, unscored: $unscored, setup_failed: (.setup_failed == true), runner_failed: (.runner_failed == true),
 caught: (if $n.type == "control" or $unscored then null else ($lines | any(startswith("missed:")) | not) end),
 outcome: (if ($lines | any(startswith("outcome: superseded"))) then "superseded" elif ($lines | any(startswith("outcome: named"))) then "named" else null end),
 over_linked: [$lines[] | scan("^over-linked: adr/([0-9]+)") | .[0] | tonumber],
 surviving_clause: ([$lines[] | scan("^surviving-clause: ([a-z]+)") | .[0]] | first),
 false_alarm: ($lines | any(startswith("false-alarm:"))), incomplete: ($lines | any(startswith("incomplete:"))),
 read_before_first_write: .usage.read_before_first_write, cost_usd: $t.cost_usd}'

readonly STATS_DEFS='
def scored: (.setup_failed | not) and (.runner_failed | not) and (.unscored | not);
def rate($part; $whole): if $whole == 0 then null else $part / $whole end;
def wilson($k; $n): if $n == 0 then null else
	(1.959963984540054) as $z | ($k / $n) as $p | (1 + $z * $z / $n) as $d
	| (($p + $z * $z / (2 * $n)) / $d) as $c
	| ($z * ((($p * (1 - $p) / $n) + ($z * $z / (4 * $n * $n))) | sqrt) / $d) as $h
	| [([0, $c - $h] | max), ([1, $c + $h] | min)] end;
def count($rows; f): $rows | map(select(f)) | length;
def summed: map(.cost_usd | select(. != null)) | if length == 0 then null else add end;
def planted_rate($rows; $arm; $size):
	[$rows[] | select(scored and .caught != null and .arm == $arm and .size == $size)] as $p
	| rate(count($p; .caught); $p | length);
def rule_of($rows; $sizes):
	planted_rate($rows; "unprompted"; $sizes[1]) as $ul | planted_rate($rows; "prompted"; $sizes[1]) as $pl
	| planted_rate($rows; "unprompted"; $sizes[0]) as $us
	| {unprompted_large: $ul, prompted_large: $pl, unprompted_small: $us,
	   build_o2: ($ul != null and (($pl != null and $ul < $pl) or ($us != null and $ul < $us)))};
def stats($all):
	[$all[] | select(scored)] as $r | [$r[] | select(.caught != null)] as $p
	| {runs: ($r | length), excluded: (($all | length) - ($r | length)), planted: ($p | length), catches: count($p; .caught),
	   catch_rate: rate(count($p; .caught); $p | length), catch_ci: wilson(count($p; .caught); $p | length),
	   superseded: count($p; .outcome == "superseded"), named: count($p; .outcome == "named"),
	   over_linked: count($r; (.over_linked | length) > 0), clause_lost: count($r; .surviving_clause == "lost"),
	   false_alarms: count($r; .false_alarm),
	   false_alarm_rate: rate(count($r; .false_alarm); $r | length), false_alarm_ci: wilson(count($r; .false_alarm); $r | length),
	   read_before_first_write: {true: count($r; .read_before_first_write == true), false: count($r; .read_before_first_write == false), null: count($r; .read_before_first_write == null)},
	   cost_usd: ($all | summed)};
def grouped(key): group_by(key | tostring) | map({key: (.[0] | key | tostring)} + stats(.));'

readonly AGGREGATE_PROGRAM="$STATS_DEFS"'
. as $rows | [$rows[] | select(.type | IN("noise-control", "x1", "exception-control") | not)] as $pooled |
([$pooled[] | select(scored) | .size] | [min, max]) as $sizes |
{rows: $rows,
 groups: {type: ($rows | grouped(.type)), size: ($pooled | grouped(.size)), wording: ($pooled | grouped(.wording)), position: ($pooled | grouped(.position)),
	arm: ($pooled | grouped(.arm)), model: ($pooled | grouped(.model)),
	cell: ($rows | grouped([.type, .size, .wording, .arm, .model] | join("/")))},
 total: {runs: ($rows | length), excluded: (count($rows; scored | not)), cost_usd: ($rows | summed)},
 rule: ({small_size: $sizes[0], large_size: $sizes[1]} + rule_of($pooled; $sizes) +
	{per_model: ([$pooled[].model] | unique | map(. as $m | {key: $m, value: rule_of([$pooled[] | select(.model == $m)]; $sizes)}) | from_entries)})}'

readonly MARKDOWN_PROGRAM='
def cell: if . == null then "n/a" else (. * 100 | round / 100 | tostring) end;
def interval: if . == null then "" else " [" + (.[0] | cell) + "–" + (.[1] | cell) + "]" end;
def table($title; $groups):
	"## " + $title, "", "| key | runs | excluded | catches | catch rate | superseded | named | over-linked | clause lost | false alarms | false alarm rate | read first | cost |",
	"| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
	($groups[] | "| \(.key) | \(.runs) | \(.excluded) | \(.catches)/\(.planted) | \(.catch_rate | cell)\(.catch_ci | interval) | \(.superseded) | \(.named) | \(.over_linked) | \(.clause_lost) | \(.false_alarms) | \(.false_alarm_rate | cell)\(.false_alarm_ci | interval) | \(.read_before_first_write.true)/\(.runs) | \(.cost_usd | cell) |"), "";
table("By type"; .groups.type), table("By size"; .groups.size), table("By wording"; .groups.wording),
table("By position"; .groups.position), table("By arm"; .groups.arm), table("By model"; .groups.model),
table("By cell (type/size/wording/arm/model)"; .groups.cell),
"## Total", "", "runs: \(.total.runs), excluded: \(.total.excluded), cost: \(.total.cost_usd | cell)", "",
"## Decision rule", "",
"unprompted at \(.rule.large_size): \(.rule.unprompted_large | cell), prompted at \(.rule.large_size): \(.rule.prompted_large | cell), unprompted at \(.rule.small_size): \(.rule.unprompted_small | cell)",
"build_o2: \(.rule.build_o2)", "",
(.rule.large_size as $large | .rule.small_size as $small | .rule.per_model | to_entries[] | "\(.key): unprompted at \($large): \(.value.unprompted_large | cell), prompted at \($large): \(.value.prompted_large | cell), unprompted at \($small): \(.value.unprompted_small | cell), build_o2: \(.value.build_o2)")'

die() {
	printf 'aggregate: %s\n' "$*" >&2
	exit 2
}

transcript_facts() { # transcript_facts <transcript>: the model and cost, or the unknowns
	if [[ -r "$1" ]]; then
		jq -R -s -c "$TRANSCRIPT_PROGRAM" "$1"
	else
		printf '{"model":"unknown","cost_usd":null}'
	fi
}

print_row() { # print_row <score.json>
	local facts
	facts="$(transcript_facts "$(dirname "$1")/transcript.jsonl")"
	jq -c --arg pattern "$NAME_PATTERN" --argjson t "$facts" "$ROW_PROGRAM" "$1" 2>/dev/null ||
		printf 'aggregate: skipped %s: not a scale scenario run\n' "$1" >&2
}

collect_rows() { # collect_rows <run dir...>
	local dir file
	for dir in "$@"; do
		for file in "$dir"/*/score.json; do
			[[ -f "$file" ]] && print_row "$file"
		done
	done
}

main() {
	local markdown=0 dirs=() dir
	command -v jq >/dev/null 2>&1 || die "jq not found on PATH"
	while (($# > 0)); do
		case "$1" in
		--markdown) markdown=1 ;;
		*) dirs+=("$1") ;;
		esac
		shift
	done
	((${#dirs[@]} > 0)) || die "usage: aggregate.sh [--markdown] RUN_DIR..."
	for dir in "${dirs[@]}"; do
		[[ -d "$dir" ]] || die "not a run directory: $dir"
	done
	local json
	json="$(collect_rows "${dirs[@]}" | jq -s -c "$AGGREGATE_PROGRAM")" || exit 1
	if ((markdown)); then jq -r "$MARKDOWN_PROGRAM" <<<"$json"; else printf '%s\n' "$json"; fi
}

main "$@"
