#!/usr/bin/env bash
#
# score.sh — score the final tree of one scratch repository and the session that produced it.
#
# usage: score.sh SCRATCH TRANSCRIPT [EXPECT]
#
# Prints one JSON object: check_ok and violations and advisories_by_kind (from
# `living-docs check --json` over SCRATCH/docs), expect_ok and expect_failures (EXPECT, run
# inside SCRATCH: exit 0 or 1, one line per failed expectation on stdout; both null when no
# EXPECT is given; EXPECT also receives the usage block as JSON in LIVING_DOCS_EVAL_USAGE), fmt_pending (from `living-docs manage fmt --check`), and usage (the
# score-transcript.sh block).
#
# LIVING_DOCS_BIN names the binary (default: target/release/living-docs of this checkout,
# then living-docs on PATH). Needs bash and jq. Exit: 0 = score printed, 2 = bad usage.

set -uo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() {
	printf 'score: %s\n' "$*" >&2
	exit 2
}

resolve_binary() {
	if [[ -n "${LIVING_DOCS_BIN:-}" ]]; then
		printf '%s' "$LIVING_DOCS_BIN"
	elif [[ -x "$HERE/../target/release/living-docs" ]]; then
		printf '%s' "$HERE/../target/release/living-docs"
	else
		command -v living-docs || true
	fi
}

[[ $# -ge 2 && $# -le 3 ]] || die "usage: score.sh SCRATCH TRANSCRIPT [EXPECT]"
SCRATCH="$1"
TRANSCRIPT="$2"
EXPECT="${3:-}"
[[ -d "$SCRATCH" ]] || die "scratch directory not found: $SCRATCH"
command -v jq >/dev/null 2>&1 || die "jq not found on PATH"
BIN="$(resolve_binary)"
[[ -n "$BIN" && -x "$BIN" ]] || die "living-docs binary not found (set LIVING_DOCS_BIN)"
BIN="$(cd "$(dirname "$BIN")" && pwd)/$(basename "$BIN")"

in_scratch() {
	(cd "$SCRATCH" && "$@")
}

check_block() {
	local out
	out="$(in_scratch "$BIN" check --json 2>&1)"
	if jq -e 'type == "object" and has("ok")' >/dev/null 2>&1 <<<"$out"; then
		jq -c '{check_ok: (.ok == true), violations: (.violations // []),
			advisories_by_kind: ((.advisories // []) | group_by(.kind // "unknown") | map({key: (.[0].kind // "unknown"), value: length}) | from_entries)}' <<<"$out"
	else
		jq -cn --arg out "$out" '{check_ok: false, violations: [{file: "", message: ("check printed no report: " + $out)}], advisories_by_kind: {}}'
	fi
}

fmt_pending() {
	local out
	out="$(in_scratch "$BIN" manage fmt --check --json docs 2>/dev/null)"
	jq -c '.pending // []' <<<"$out" 2>/dev/null || printf '[]'
}

expect_block() { # expect_block <usage json>
	if [[ -z "$EXPECT" ]]; then
		printf '{"expect_ok":null,"expect_failures":null}'
		return
	fi
	local out rc
	out="$(PATH="$(dirname "$BIN"):$PATH" LIVING_DOCS_EVAL_USAGE="$1" in_scratch bash "$EXPECT" 2>&1)"
	rc=$?
	if ((rc != 0)) && [[ -z "$out" ]]; then
		out="expect.sh exited $rc with no message"
	fi
	jq -cn --arg out "$out" --argjson rc "$rc" \
		'{expect_ok: ($rc == 0), expect_failures: ($out | split("\n") | map(select(length > 0)))}'
}

usage_block() {
	local out
	out="$(bash "$HERE/score-transcript.sh" "$TRANSCRIPT" 2>/dev/null)"
	if jq -e 'type == "object"' >/dev/null 2>&1 <<<"$out"; then
		printf '%s' "$out"
	else
		printf '{"transcript_recognized":false}'
	fi
}

USAGE="$(usage_block)"
jq -cn \
	--argjson check "$(check_block)" \
	--argjson expect "$(expect_block "$USAGE")" \
	--argjson fmt "$(fmt_pending)" \
	--argjson usage "$USAGE" \
	'$check + $expect + {fmt_pending: $fmt, usage: $usage}'
