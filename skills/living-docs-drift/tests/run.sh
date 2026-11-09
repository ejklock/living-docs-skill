#!/usr/bin/env bash
#
# run.sh — fixture tests for scripts/drift.sh, driven only by stub judge commands under
# fixtures/judges. No real model is ever called; every judge answer is a fixed file under
# fixtures/answers. Scratch state lives under $TMPDIR and is removed on exit.
#
# Exit: 0 = all cases pass, 1 = at least one failed.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
DRIFT="$HERE/../scripts/drift.sh"
PROMPT="$HERE/../prompt.md"
FIX="$HERE/fixtures"
BASH_BIN="${BASH:-$(command -v bash)}"
TMP="$(mktemp -d "${TMPDIR:-/tmp}/living-docs-drift-test.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT

fail=0
case_no=0

resolve_living_docs() {
	if [[ -n "${LIVING_DOCS_BIN:-}" ]]; then
		printf '%s' "$LIVING_DOCS_BIN"
	elif [[ -x "$HERE/../../../target/release/living-docs" ]]; then
		printf '%s' "$HERE/../../../target/release/living-docs"
	else
		command -v living-docs || true
	fi
}
LIVING_DOCS="$(resolve_living_docs)"

pass() { printf '  ok    %s\n' "$1"; }
flunk() {
	printf '  FAIL  %s — %s\n' "$1" "$2"
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

assert_false() { # assert_false <name> <command...>
	local name="$1"
	shift
	if "$@"; then flunk "$name" "condition true: $*"; else pass "$name"; fi
}

new_case() {
	case_no=$((case_no + 1))
	CASE="$TMP/case-$case_no"
	ROOT="$CASE/root"
	STUB_DIR="$CASE/stub"
	OUT="$CASE/out"
	mkdir -p "$ROOT" "$STUB_DIR"
	cp -R "$FIX/src" "$ROOT/src"
	printf 'SECRET-OUTSIDE-BYTES\n' >"$CASE/outside.rs"
	ln -s "$CASE/outside.rs" "$ROOT/src/link.rs"
	JUDGE="$BASH_BIN '$FIX/judges/record.sh'"
	TIMEOUT=""
	DRIVE_PATH="$PATH"
	export STUB_DIR
}

drive() { # drive <docs-dir> [extra driver args...]; sets RC, STDERR_FILE
	local docs="$1"
	shift
	local envargs=(-u LIVING_DOCS_DRIFT_JUDGE -u LIVING_DOCS_DRIFT_TIMEOUT "PATH=$DRIVE_PATH")
	[[ -n "$JUDGE" ]] && envargs+=("LIVING_DOCS_DRIFT_JUDGE=$JUDGE")
	[[ -n "$TIMEOUT" ]] && envargs+=("LIVING_DOCS_DRIFT_TIMEOUT=$TIMEOUT")
	STDERR_FILE="$CASE/stderr"
	env "${envargs[@]}" "$BASH_BIN" "$DRIFT" --docs-dir "$docs" --root "$ROOT" ${OUT:+--out "$OUT"} "$@" \
		>"$CASE/stdout" 2>"$STDERR_FILE"
	RC=$?
}

report_files() { find "$OUT" -name '*-drift-report.md' 2>/dev/null | LC_ALL=C sort; }
report_file() { report_files | head -n 1; }

payload_of() { awk 'f{print} /^-----BEGIN PAYLOAD/{f=1}' "$1"; }

sha_of() {
	if command -v sha256sum >/dev/null 2>&1; then sha256sum | awk '{print $1}'; else shasum -a 256 | awk '{print $1}'; fi
}

section_of() { # section_of <report> <record path>
	awk -v h="## $2" '$0==h{f=1;next} /^## /{f=0} f' "$1"
}

recorded() { find "$STUB_DIR" -name '*.in' | LC_ALL=C sort; }

# A missing pidfile must never become pid 0: kill 0 signals this runner's own process group.
assert_stub_gone() { # assert_stub_gone <case name> <pidfile>
	local name="$1: $2 process is gone" pid
	pid="$(cat "$STUB_DIR/$2" 2>/dev/null || true)"
	if [[ ! "$pid" =~ ^[1-9][0-9]*$ ]]; then
		flunk "$name" "pidfile missing or empty"
	elif kill -0 "$pid" 2>/dev/null; then
		kill -KILL "$pid" 2>/dev/null
		flunk "$name" "pid $pid still alive"
	else
		pass "$name"
	fi
}

echo "living-docs-drift driver fixtures"
echo

# Scope by record type and status.
new_case
drive "$FIX/bundle"
assert_eq "scope: run exits 0" "0" "$RC"
modes="$(
	for f in $(recorded); do payload_of "$f" | jq -r '[.record.path, .mode] | join(" ")'; done | LC_ALL=C sort
)"
assert_eq "scope: only in-scope records are sent, each in its mode" \
	"adr/0003-accepted-choice.md adherence
architecture/compute.md anchors
constitution.md anchors
issues/0001-open-slice.md anchors" "$modes"
nulls="$(
	for f in $(recorded); do payload_of "$f" | jq -r 'select(.record.type == "Constitution" or .record.type == "Architecture View") | .record.number'; done
)"
assert_eq "scope: view and constitution carry a null number" "null
null" "$nulls"

new_case
drive "$FIX/docs-closed"
assert_eq "scope: a bundle with nothing in scope exits 0" "0" "$RC"
assert_eq "scope: nothing in scope sends no payload" "0" "$(recorded | wc -l | tr -d ' ')"
assert_true "scope: the report says nothing was in scope" grep -q 'No in-scope records' "$(report_file)"

# Anchor resolution.
new_case
drive "$FIX/docs-anchors"
payload="$(recorded | head -n 1)"
assert_eq "anchors: run exits 0" "0" "$RC"
assert_eq "anchors: each cited path resolves as the rules say" \
	'[{"path":"src/a.rs","symbol":"compute","resolution":"present"},{"path":"src/a.rs","symbol":"comp","resolution":"name_absent"},{"path":"src/gone.rs","symbol":null,"resolution":"path_missing"},{"path":"src/b.rs","symbol":"helper","resolution":"present"},{"path":"src/c.txt","symbol":null,"resolution":"present"},{"path":"src/long.rs","symbol":null,"resolution":"present"},{"path":"/etc/hosts","symbol":null,"resolution":"unresolvable"},{"path":"../outside.rs","symbol":null,"resolution":"unresolvable"},{"path":"src/link.rs","symbol":null,"resolution":"unresolvable"}]' \
	"$(payload_of "$payload" | jq -c .anchors)"
assert_eq "anchors: only readable files get an excerpt" \
	'["src/a.rs","src/b.rs","src/c.txt","src/long.rs"]' \
	"$(payload_of "$payload" | jq -c '[.excerpts[].path]')"
assert_eq "anchors: an excerpt past 200 lines is cut and flagged" "true 200" \
	"$(payload_of "$payload" | jq -r '.excerpts[] | select(.path == "src/long.rs") | "\(.truncated) \(.content | split("\n") | map(select(length > 0)) | length)"')"
assert_eq "anchors: a short excerpt is whole and not flagged" "false" \
	"$(payload_of "$payload" | jq -r '.excerpts[] | select(.path == "src/a.rs") | .truncated')"
assert_false "anchors: bytes of files outside the root never reach the payload" grep -q 'SECRET-OUTSIDE-BYTES' "$payload"
assert_false "anchors: an absolute path is never read" grep -q 'localhost' "$payload"
assert_false "anchors: a fenced anchor is ignored" \
	grep -q fenced.rs <<<"$(payload_of "$payload" | jq -c .anchors)"
assert_false "anchors: a backticked word that is not a path is ignored" grep -q '"cargo test"' "$payload"

new_case
drive "$FIX/docs-tokens"
assert_eq "anchors: a bare extension is no anchor, a dotted path is" \
	'["README.md","src/a.rs",".github/workflows/ci.yml","src/.hidden.rs"]' \
	"$(payload_of "$(recorded | head -n 1)" | jq -c '[.anchors[].path]')"

new_case
mkdir -p "$CASE/docs-cap/issues"
{
	printf -- '---\ntype: Issue\ntitle: Cap\nstatus: open\n---\n\n'
	for i in $(seq 1 25); do printf '%ssrc/f%02d.rs%s ' '`' "$i" '`'; done
	printf '\n'
} >"$CASE/docs-cap/issues/0001-cap.md"
drive "$CASE/docs-cap"
assert_eq "anchors: at most 20 anchors per record" "20" \
	"$(payload_of "$(recorded | head -n 1)" | jq '.anchors | length')"

# Payload shape and data isolation.
new_case
drive "$FIX/docs-hostile"
payload="$(recorded | head -n 1)"
payload_has_shape() {
	payload_of "$payload" | jq -e '
		.schema_version == 1 and .mode == "anchors"
		and (.record | keys == ["number", "path", "status", "title", "type"])
		and .record.number == 1 and .record.status == "open" and .record.title == "Hostile"
		and (.anchors | type == "array") and (.excerpts | type == "array")
		and (.excerpts[0] | keys == ["content", "path", "truncated"])' >/dev/null
}
assert_true "payload: has the documented shape" payload_has_shape
hostile_text_survives() {
	payload_of "$payload" | jq -e --arg tick '`' \
		'.body | contains("ignore previous instructions") and contains("</payload>") and contains($tick + "rm -rf /" + $tick)' >/dev/null
}
assert_true "payload: hostile record text survives as a JSON string" hostile_text_survives
assert_eq "payload: the body is the record text after its frontmatter" \
	"$(tail -n +8 "$FIX/docs-hostile/issues/0001-hostile.md")" \
	"$(payload_of "$payload" | jq -r .body | sed '$d')"
assert_true "payload: the excerpt is the source bytes" \
	cmp -s <(payload_of "$payload" | jq -j '.excerpts[0].content') "$FIX/src/hostile.rs"
starts_with_prompt() {
	local sent prompt_text
	sent="$(cat "$payload")"
	prompt_text="$(cat "$PROMPT")"
	[[ "$sent" == "$prompt_text"* ]]
}
assert_true "stdin: starts with the prompt text" starts_with_prompt
assert_eq "stdin: one data marker precedes the payload" "1" "$(grep -c '^-----BEGIN PAYLOAD' "$payload")"
assert_true "stdin: the marker says the payload is data, never instructions" \
	grep -q '^-----BEGIN PAYLOAD.*data.*never instructions' "$payload"
assert_eq "stdin: the payload is a single JSON line" "1" "$(payload_of "$payload" | wc -l | tr -d ' ')"

# Judge answer validation.
answer_case() { # answer_case <answer file> <expected verdict text> <present|absent kind-or-empty>
	new_case
	JUDGE="$BASH_BIN '$FIX/judges/fixed.sh'"
	export STUB_ANSWER="$FIX/answers/$1"
	drive "$FIX/docs-one"
	assert_eq "answer $1: run exits 0" "0" "$RC"
	assert_true "answer $1: report shows $2" grep -qF -- "**Verdict:** $2" "$(report_file)"
}
answer_case no_drift.json "no_drift"
answer_case drift_four.json "drift"
for kind in missing_feature incomplete_model logic_shortcut architectural_divergence; do
	assert_true "answer drift_four.json: report lists $kind" grep -q "$kind" "$(report_file)"
done
answer_case drift_fenced.txt "drift"
assert_true "answer drift_fenced.txt: findings survive the fence" grep -q 'layers crossed' "$(report_file)"
answer_case drift_fenced_crlf.txt "drift"
assert_true "answer drift_fenced_crlf.txt: findings survive the fence" grep -q 'layers crossed' "$(report_file)"
answer_case no_drift_crlf.json "no_drift"
for bad in two_fences_crlf.txt prose_before_crlf.txt; do
	answer_case "$bad" "invalid answer"
	assert_false "answer $bad: never reported as no drift" grep -q 'Verdict:\*\* no_drift' "$(report_file)"
done
for bad in drift_no_findings.json no_drift_with_finding.json unknown_kind.json schema_two.json \
	empty_summary.json empty_path.json prose_before.txt two_fences.txt empty.txt; do
	answer_case "$bad" "invalid answer"
	assert_false "answer $bad: never reported as no drift" grep -q 'Verdict:\*\* no_drift' "$(report_file)"
done

# Fail direction.
expect_failure() { # expect_failure <name> <stderr substring>
	assert_eq "$1: exits 2" "2" "$RC"
	assert_eq "$1: one stderr line" "1" "$(wc -l <"$STDERR_FILE" | tr -d ' ')"
	assert_true "$1: stderr names the cause" grep -qF -- "$2" "$STDERR_FILE"
	assert_eq "$1: no report file" "" "$(report_files)"
}

new_case
JUDGE="no-such-judge-command --flag"
drive "$FIX/bundle"
expect_failure "missing judge command" "judge command not found: no-such-judge-command"

new_case
JUDGE="$BASH_BIN '$FIX/judges/fail-second.sh'"
drive "$FIX/bundle"
expect_failure "judge exits non-zero on the second record" "judge exited with status 1"
assert_eq "judge exits non-zero: the first record was already answered" "2" \
	"$(find "$STUB_DIR" -name 'call-*' | wc -l | tr -d ' ')"

new_case
JUDGE="$BASH_BIN '$FIX/judges/sleep.sh'"
TIMEOUT=1
SECONDS=0
drive "$FIX/docs-one"
elapsed=$SECONDS
expect_failure "judge past the timeout" "judge timed out after 1s"
assert_true "judge past the timeout: the run returns within a few seconds" test "$elapsed" -lt 6
assert_stub_gone "judge past the timeout" judge.pid
assert_stub_gone "judge past the timeout" sleep.pid

# Interrupt: the judge and its children die with the driver, and no report is written.
interrupt_case() { # interrupt_case <signal>
	local sig="$1" driver_pid i
	new_case
	JUDGE="$BASH_BIN '$FIX/judges/sleep.sh'"
	TIMEOUT=1000
	set -m # a background job of a non-interactive shell would ignore SIGINT
	env -u LIVING_DOCS_DRIFT_JUDGE -u LIVING_DOCS_DRIFT_TIMEOUT "PATH=$DRIVE_PATH" \
		"LIVING_DOCS_DRIFT_JUDGE=$JUDGE" "LIVING_DOCS_DRIFT_TIMEOUT=$TIMEOUT" \
		"$BASH_BIN" "$DRIFT" --docs-dir "$FIX/docs-one" --root "$ROOT" --out "$OUT" \
		>"$CASE/stdout" 2>"$CASE/stderr" &
	driver_pid=$!
	set +m
	for i in $(seq 1 50); do
		[[ -s "$STUB_DIR/sleep.pid" ]] && break
		sleep 0.1
	done
	kill "-$sig" "$driver_pid" 2>/dev/null
	for i in $(seq 1 50); do
		kill -0 "$driver_pid" 2>/dev/null || break
		sleep 0.1
	done
	if kill -0 "$driver_pid" 2>/dev/null; then
		kill -KILL "$driver_pid" 2>/dev/null
		flunk "$sig: the driver exits within 5s" "pid $driver_pid still alive"
		RC=0
	else
		RC=0
		wait "$driver_pid" || RC=$?
		pass "$sig: the driver exits within 5s"
	fi
	assert_true "$sig: the driver exits non-zero" test "$RC" -ne 0
	assert_stub_gone "$sig" judge.pid
	assert_stub_gone "$sig" sleep.pid
	assert_eq "$sig: no report file" "" "$(report_files)"
}
interrupt_case TERM
interrupt_case INT

new_case
mkdir -p "$CASE/nojq-bin"
for tool in bash sh cat awk sed grep sort head tail wc tr date mktemp dirname basename sleep readlink \
	shasum sha256sum cut rm mv mkdir cp ls find uniq env kill; do
	found="$(command -v "$tool" 2>/dev/null || true)"
	[[ -n "$found" && "$found" == /* ]] && ln -s "$found" "$CASE/nojq-bin/$tool"
done
DRIVE_PATH="$CASE/nojq-bin"
drive "$FIX/bundle"
expect_failure "jq absent" "jq not found"

# jq floor.
jq_stub_case() { # jq_stub_case <version string>
	new_case
	mkdir -p "$CASE/jq-bin"
	printf '#!/usr/bin/env bash\nif [[ "${1:-}" == "--version" ]]; then echo "%s"; exit 0; fi\nexec "%s" "$@"\n' \
		"$1" "$(command -v jq)" >"$CASE/jq-bin/jq"
	chmod +x "$CASE/jq-bin/jq"
	DRIVE_PATH="$CASE/jq-bin:$PATH"
	drive "$FIX/docs-one"
}
for old in jq-1.5 jq-0.9 jq-master; do
	jq_stub_case "$old"
	expect_failure "jq $old" "jq 1.6 or newer"
done
for ok in jq-1.6 jq-1.7.1; do
	jq_stub_case "$ok"
	assert_eq "jq $ok: run exits 0" "0" "$RC"
done
new_case
drive "$FIX/docs-one"
assert_eq "jq on the machine: run exits 0" "0" "$RC"

new_case
drive "$CASE/no-such-docs"
expect_failure "docs directory missing" "docs directory not found"

new_case
drive "$FIX/bundle" --bogus
assert_eq "unknown flag: exits 2" "2" "$RC"
assert_true "unknown flag: prints usage" grep -qi 'usage' "$STDERR_FILE"
assert_eq "unknown flag: no report file" "" "$(report_files)"

# Report.
new_case
today_before="$(date +%Y-%m-%d)"
drive "$FIX/bundle"
today_after="$(date +%Y-%m-%d)"
report="$(report_file)"
assert_true "report: named by date under the out directory" \
	test "$report" = "$OUT/$today_before-drift-report.md" -o "$report" = "$OUT/$today_after-drift-report.md"
assert_eq "report: no stray temp file beside it" "1" "$(find "$OUT" -type f | wc -l | tr -d ' ')"
assert_true "report: header names the judge command line" grep -qF -- "$JUDGE" "$report"
for f in $(recorded); do
	rec="$(payload_of "$f" | jq -r .record.path)"
	assert_eq "report: $rec section carries the payload hash" "$(payload_of "$f" | sha_of)" \
		"$(section_of "$report" "$rec" | awk '/Payload SHA-256/{print $NF}')"
done
assert_true "report: an anchors section lists anchor results" \
	grep -q "src/missing-issue.rs.*path_missing" <<<"$(section_of "$report" issues/0001-open-slice.md)"
assert_false "report: an adherence section never lists anchor results" \
	grep -q "missing-adr\|path_missing" <<<"$(section_of "$report" adr/0003-accepted-choice.md)"
OUT=""
drive "$FIX/docs-one"
OUT="$ROOT/.living-docs/drift"
assert_eq "report: defaults to the root's .living-docs/drift" "1" \
	"$(report_files | wc -l | tr -d ' ')"

# Report directory stays out of git.
new_case
git init -q "$ROOT"
OUT=""
drive "$FIX/docs-one"
OUT="$ROOT/.living-docs/drift"
assert_eq "git: the default report directory ignores itself" "*" "$(cat "$OUT/.gitignore" 2>/dev/null)"
assert_eq "git: nothing under .living-docs shows in status" "" \
	"$(git -C "$ROOT" status --porcelain --untracked-files=all -- .living-docs)"
assert_eq "git: the report is still written" "1" "$(report_files | wc -l | tr -d ' ')"

new_case
mkdir -p "$OUT"
printf 'keep-me\n!report\n' >"$OUT/.gitignore"
cp "$OUT/.gitignore" "$CASE/gitignore.before"
drive "$FIX/docs-one"
assert_true "git: an existing .gitignore in the report directory is left as it was" \
	cmp -s "$CASE/gitignore.before" "$OUT/.gitignore"
assert_eq "git: with an existing .gitignore the report is still written" "1" "$(report_files | wc -l | tr -d ' ')"

new_case
JUDGE="$BASH_BIN '$FIX/judges/fixed.sh'"
export STUB_ANSWER="$FIX/answers/drift_four.json"
drive "$FIX/docs-one"
export STUB_ANSWER="$FIX/answers/no_drift.json"
drive "$FIX/docs-one"
assert_eq "report: a second run the same day leaves one report" "1" "$(report_files | wc -l | tr -d ' ')"
assert_false "report: the second run replaces the first run's content" grep -q 'feature absent' "$(report_file)"

new_case
mkdir -p "$CASE/claude-bin"
printf '#!/usr/bin/env bash\ncat >/dev/null\nprintf "%%s\\n" "{\\"schema_version\\":1,\\"verdict\\":\\"no_drift\\",\\"findings\\":[]}"\n' >"$CASE/claude-bin/claude"
chmod +x "$CASE/claude-bin/claude"
JUDGE=""
DRIVE_PATH="$CASE/claude-bin:$PATH"
drive "$FIX/docs-one"
assert_eq "default judge: run exits 0" "0" "$RC"
assert_true "default judge: header names claude -p with no tools" grep -qF -- 'claude -p --tools ""' "$(report_file)"

new_case
cp -R "$FIX/bundle" "$CASE/bundle-copy"
drive "$CASE/bundle-copy"
assert_eq "bundle copy: run exits 0" "0" "$RC"
if [[ -z "$LIVING_DOCS" ]]; then
	flunk "bundle copy: living-docs check stays green" "no living-docs binary found"
else
	"$LIVING_DOCS" check --plain --docs-dir "$CASE/bundle-copy" >/dev/null 2>&1
	assert_eq "bundle copy: living-docs check stays green" "0" "$?"
fi
assert_true "bundle copy: the run left the bundle untouched" diff -r "$FIX/bundle" "$CASE/bundle-copy"

echo
if ((fail == 0)); then echo "all drift fixtures passed"; else echo "drift fixtures FAILED"; fi
exit "$fail"
