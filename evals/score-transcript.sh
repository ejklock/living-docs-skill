#!/usr/bin/env bash
#
# score-transcript.sh — how a session used the living-docs CLI, from its transcript alone.
#
# Reads a Claude Code JSONL transcript (one JSON object per line; assistant lines hold
# tool_use items, user lines hold tool_result items) and prints one JSON object with the usage
# block: cli_calls, cli_calls_by_verb, cli_errors, unknown_subcommand, help_probes,
# bare_number_refs, read_before_first_write, owned_field_hand_edits. A file in another shape,
# an empty file or a missing file yields transcript_recognized false and every count null.
#
# Needs bash and jq. Exit: 0 = block printed, 2 = bad usage.

set -euo pipefail
export LC_ALL=C

[[ $# -eq 1 ]] || {
	echo "usage: score-transcript.sh TRANSCRIPT" >&2
	exit 2
}
command -v jq >/dev/null 2>&1 || {
	echo "score-transcript: jq not found on PATH" >&2
	exit 2
}

readonly PROGRAM='
def text_of: if type == "string" then . elif type == "array" then map(.text? // "" | tostring) | join("\n") else "" end;

def strip_heredocs:
	split("\n")
	| reduce .[] as $line ({term: null, out: []};
		if .term != null then
			(if ($line | sub("^\\t+"; "")) == .term then .term = null else . end)
		else
			.out += [$line]
			| (($line | capture("(?:^|[^<])<<-?[ \\t]*[\u0027\"]?(?<t>[A-Za-z_][A-Za-z0-9_]*)[\u0027\"]?") // null) as $m
				| if $m then .term = $m.t else . end)
		end)
	| .out | join("\n");

def ch: [.] | implode;

def flush: if .has then .words += [.cur] | .cur = "" | .has = false else . end;

def end_segment: flush | if (.words | length) > 0 then .segs += [.words] | .words = [] else . end;

def step($c):
	if .comment then (if $c == 10 then .comment = false | end_segment else . end)
	elif .esc then (if $c == 10 then .esc = false else .cur += ($c | ch) | .has = true | .esc = false end)
	elif .q == 39 then (if $c == 39 then .q = 0 else .cur += ($c | ch) end)
	elif .q == 34 then (if $c == 34 then .q = 0 elif $c == 92 then .esc = true else .cur += ($c | ch) end)
	elif $c == 39 then .q = 39 | .has = true
	elif $c == 34 then .q = 34 | .has = true
	elif $c == 92 then .esc = true
	elif $c == 35 and (.has | not) then .comment = true
	elif $c == 32 or $c == 9 then flush
	elif $c == 10 or $c == 59 or $c == 38 or $c == 124 or $c == 40 or $c == 41 or $c == 96 then end_segment
	else .cur += ($c | ch) | .has = true
	end;

def segments:
	(strip_heredocs + "\n" | explode) as $codes
	| reduce $codes[] as $c ({segs: [], words: [], cur: "", has: false, q: 0, esc: false, comment: false}; step($c))
	| .segs;

def drop_prefix:
	until(length == 0 or ((.[0] | test("^[A-Za-z_][A-Za-z0-9_]*=") or IN("env", "time", "command", "exec", "sudo", "nohup")) | not); .[1:]);

def positionals:
	reduce .[] as $a ({p: [], skip: false};
		if .skip then .skip = false
		elif ($a | test("^--(docs-dir|color|from|body|description|owner|kind|topic|links-to|impact)$")) then .skip = true
		elif ($a | startswith("-")) then .
		else .p += [$a] end)
	| .p;

def flag_values: . as $args | [range(0; length - 1) as $i | select($args[$i] | IN("--links-to", "--impact")) | $args[$i + 1]];

def parse_call:
	.[1:] as $args
	| ($args | positionals) as $p
	| (if ($p[0] // "") | IN("write", "read", "manage") then ($p[0] + " " + ($p[1] // "")) else ($p[0] // "(none)") end) as $verb
	| ((if $verb == "write set" then $p[2:3] elif $verb == "write supersede" then $p[2:4] else [] end) + ($args | flag_values)) as $refs
	| {verb: $verb,
		help: (($args | any(. == "--help" or . == "-h")) or (($p[0] // "") == "help")),
		bare: ($refs | map(select(test("^[0-9]+$"))) | length)};

def cli_calls_of: segments | map(drop_prefix | select(length > 0 and (.[0] == "living-docs" or (.[0] | endswith("/living-docs")))) | parse_call);

def bundle_record: type == "string" and test("(^|/)docs/.+\\.md$");
def is_index: test("(^|/)index\\.md$");

def strong_key_line: test("^(status|superseded_by|supersedes|timestamp|owner|tracker|blocked_by):") or test("^type:[ \\t]*(ADR|Issue|PRD|Research|View|Constitution)[ \\t]*$");
def weak_key_line: test("^(type|title|description|labels|number):");
def delimiter_line: test("^---[ \\t]*$");
def callout_line: test("^> \\*\\*(SUPERSEDED|DEPRECATED)");
def opens_frontmatter: (.[0] | delimiter_line) and ((.[1] // "") | (strong_key_line or weak_key_line));
def touches_owned_text: split("\n") as $lines
	| ($lines | opens_frontmatter) or ($lines | any(callout_line or strong_key_line)) or ([$lines[] | select(weak_key_line)] | length >= 2);

def edit_pairs:
	if .name == "MultiEdit" then (.input.edits | if type == "array" then map(objects | [((.old_string // "") | tostring), ((.new_string // "") | tostring)]) else [] end)
	else [[((.input.old_string // "") | tostring), ((.input.new_string // "") | tostring)]] end;

def hand_edit:
	(.input.file_path // "") as $path
	| if ($path | bundle_record) | not then false
	elif ($path | is_index) then true
	elif .name == "Write" then ((.input.content // "") | tostring | (test("^---[ \\t]*\n") or (split("\n") | any(callout_line))))
	else (edit_pairs | any(.[] | touches_owned_text)) end;

def errored($result; $text): ($result.is_error == true) or ($text | test("^Exit code:? *[1-9]"));

def read_before_write($events):
	($events | index("read")) as $first_read
	| ($events | index("write")) as $first_write
	| if $first_read == null and $first_write == null then null
	elif $first_read == null then false
	else ($first_write == null or $first_read < $first_write) end;

def block:
	[.[] | select(type == "object" and (.type == "assistant" or .type == "user") and (((.message.content? // null) | type) == "array"))] as $messages
	| if ($messages | length) == 0 then
		{transcript_recognized: false, cli_calls: null, cli_calls_by_verb: null, cli_errors: null, unknown_subcommand: null,
			help_probes: null, bare_number_refs: null, read_before_first_write: null, owned_field_hand_edits: null}
	else
		[$messages[] | .message.content[] | objects] as $items
		| ($items | map(select(.type == "tool_result" and (.tool_use_id | type) == "string") | {key: .tool_use_id, value: .}) | from_entries) as $results
		| [$items[] | select(.type == "tool_use") | .input = (if (.input | type) == "object" then .input else {} end) | .id = (if (.id | type) == "string" then .id else "" end) | .name = (if (.name | type) == "string" then .name else "" end)] as $uses
		| [$uses[] | select(.name == "Bash") | {id: .id, calls: ((.input.command // "") | tostring | cli_calls_of)} | select(.calls | length > 0)] as $bash
		| ($bash | map(.calls[])) as $calls
		| ($bash | map(($results[.id] // {}) as $r | ($r.content | text_of) as $text
			| errored($r; $text) as $err
			| {err: $err, unknown: ($err and ($text | test("unrecognized subcommand")))})) as $outcomes
		| [$uses[] | if .name == "Bash" then ((.input.command // "") | tostring | cli_calls_of | map(if (.verb | startswith("read ")) then "read" elif (.verb | startswith("write ")) then "write" else empty end)[])
			elif (.name | IN("Edit", "Write", "MultiEdit")) and ((.input.file_path // "") | bundle_record) then "write" else empty end] as $events
		| {transcript_recognized: true,
			cli_calls: ($calls | length),
			cli_calls_by_verb: ($calls | group_by(.verb) | map({key: .[0].verb, value: length}) | from_entries),
			cli_errors: ($outcomes | map(select(.err)) | length),
			unknown_subcommand: ($outcomes | map(select(.unknown)) | length),
			help_probes: ($calls | map(select(.help)) | length),
			bare_number_refs: ($calls | map(.bare) | add // 0),
			read_before_first_write: read_before_write($events),
			owned_field_hand_edits: ([$uses[] | select(.name | IN("Edit", "Write", "MultiEdit")) | select(hand_edit)] | length)}
	end;

split("\n") | map(try fromjson catch empty) | block
'

if [[ -r "$1" ]]; then
	jq -R -s -c "$PROGRAM" "$1"
else
	printf '' | jq -R -s -c "$PROGRAM"
fi
