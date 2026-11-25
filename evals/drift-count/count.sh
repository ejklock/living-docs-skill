#!/usr/bin/env bash
#
# count.sh — count the decisions in a log that the code or the world may have left behind.
#
# usage: count.sh [--markdown] [--docs-dir NAME] [--sample N] [--seed S] REPO...
#
# For each git repository it reads the ADRs in force under REPO/NAME/adr (NAME defaults to docs):
# a file NNNN-*.md whose frontmatter has type ADR and status Accepted, dated by its frontmatter
# timestamp (YYYY-MM-DD read as midnight UTC, or ISO 8601 with Z, a +HH:MM offset or none, read as
# UTC). An ADR whose timestamp is missing or unparseable is counted in no_timestamp and is no
# candidate of any type. Instants are compared as epoch seconds.
# The impact paths of an ADR are the backticked tokens of its whole **Implementation impact:**
# field (the marker to the next blank line or the next **Field:** marker) that look like
# repository paths (a "/" in them, or a file extension, but not a version such as 0.34.0). A brace
# group (src/{a.rs, b.rs}) expands. A token resolves, against the files at HEAD, to the file, the
# files under a directory, the files a glob matches, a path that existed in history, or the one
# file that ends with the token (a crate-relative path). A token that resolves to nothing, or by
# suffix to several files, is listed in unresolved; an ADR whose tokens all fail is counted in
# unresolved_adrs and is neither t3 nor t4. The rule applies to each resolved file. Candidates:
#   t3  a file has 3 or more non-merge commits with a committer instant after the ADR's, and no
#       newer record under NAME names that file: its exact path as a word or a whole backticked span
#       (a trailing :line, :line:col, #Lline, ::item or (args) is dropped) anywhere in the record, or
#       a directory, glob or crate-relative token in the record's own Implementation impact field
#       that resolves to it (globs follow git pathspec glob rules)
#   t4  no resolved file has a commit after the ADR's instant
#   t5  the ADR body names a version (v1.2, v2.3.1, a bare 3.9.0, a product name followed by a number such as
#       Node 20 or Python 3.11, an image tag in backticks such as postgres:15), a price (a currency
#       symbol or code with a number), or a quota or vendor limit (a number with KB, MB, GB, TB,
#       req/s, /month, /day, per month, or a number after "rate limit" or "quota"); the reason
#       records the matched text. A token read as an impact path is not scanned.
# An ADR with no impact path is counted in no_impact_paths and is neither a t3 nor a t4 candidate.
# Prints one JSON object: repos (one per argument: repo, head, docs_dir, adrs_in_force,
# no_impact_paths, no_impact_adrs, no_timestamp, no_timestamp_adrs, unresolved_adrs, unresolved as
# {number, token}, t3, t4, t5 as {number, title, reason} sorted by number, and counts of the three)
# and total. --sample N adds sample: per type up to N candidates pooled over every repository,
# chosen by a shuffle seeded with --seed (default 0), each with repo, number, title and reason.
# --markdown prints a table per repository and a total line. It never calls a model and never
# writes into a repository. Needs bash, jq, git and awk.
# Exit: 0 = printed, 2 = bad usage or an unreadable repository.

set -uo pipefail
export LC_ALL=C
export GIT_OPTIONAL_LOCKS=0

readonly IMPACT_MARKER='**Implementation impact:**'
readonly MIN_COMMITS=3
scratch=

readonly EPOCH_PROGRAM='
try ($s | capture("^(?<d>[0-9]{4}-[0-9]{2}-[0-9]{2})(?:[T ](?<t>[0-9]{2}:[0-9]{2}:[0-9]{2})(?:\\.[0-9]+)?(?<z>Z|[+-][0-9]{2}:?[0-9]{2})?)?$")
	| ((.d + "T" + (.t // "00:00:00") + "Z") | strptime("%Y-%m-%dT%H:%M:%SZ") | mktime) as $base
	| (.z // "Z") as $z
	| if $z == "Z" then $base
		else (($z[1:3] | tonumber) * 3600 + ($z[-2:] | tonumber) * 60) as $off
			| if $z[0:1] == "-" then $base + $off else $base - $off end
		end) catch empty'

readonly VALUES_PROGRAM='
def num: "[0-9]+(?:,[0-9]{3})*(?:\\.[0-9]+)?";
def edge: "(?![A-Za-z0-9_/-]|\\.[A-Za-z0-9])";
def labels: ["adr","issue","stage","section","step","phase","slice","table","figure","fig","part","chapter","rule","item","option","case","round",
	"day","week","month","year","task","line","lines","page","note","level","tier","criterion","layer","test","pr","pull","epic","story","decision",
	"question","row","column","file","of","about","exit","exits","at","in","on","for","to","by","from","with","after","before","over","under",
		"since","until","into","than","see","and","or","but","if","when","then","the","a","an","as","is","it","this","that","these","those","each",
		"every","all","some","any","no","not","only","also","january","february","march","april","may","june","july","august","september","october","november","december"];
def versions: scan("(?<![A-Za-z0-9_./-])v[0-9]+(?:\\.[0-9]+){0,2}" + edge),
	scan("(?<![A-Za-z0-9_./-])[0-9]+\\.[0-9]+\\.[0-9]+" + edge),
	(scan("(?<![A-Za-z0-9_./-])[A-Z][A-Za-z]*(?:\\.[a-z]+)? [0-9]{1,3}(?:\\.[0-9]+){0,2}" + edge + "(?!%| ?(?:[KMGT]B|ms|s|req)(?![A-Za-z]))")
		| select((split(" ")[0] | ascii_downcase) as $w | labels | index($w) | not)),
	scan("(?<=`)[a-z][a-z0-9_-]*(?:/[a-z0-9_-]+)*:[0-9]+(?:\\.[0-9]+)*(?=`)");
def prices: scan("(?<![A-Za-z0-9_`])[$€£¥] ?" + num + "(?![0-9`]|\\.[0-9])(?: ?(?:per |/)(?:month|day|year|user|seat|hour))?"),
	scan("\\b(?:USD|EUR|BRL|GBP) ?" + num),
	scan(num + " ?(?:USD|EUR|BRL|GBP)\\b");
def quotas: scan(num + " ?(?:MB|GB|TB|KB|req/s)(?![A-Za-z])"; "i"),
	scan(num + " ?(?:/month|/day|per month)"; "i"),
	scan("(?:rate limit|quota)(?: of| is|:)? " + num + "(?: ?(?:req/s|[KMGT]B|/month|/day|per month))?(?![A-Za-z])"; "i");
($tokens | split("\n") | map(select(length > 0))) as $skip
| (reduce $skip[] as $t ($body; split("`" + $t + "`") | join(" "))) as $text
| [$text | versions, prices, quotas] as $found
| ($found | reduce .[] as $m ([]; if index([$m]) then . else . + [$m] end)) as $unique
| $unique | map(select(. as $m | [$unique[] | select(contains($m) and . != $m)] | length == 0))'

readonly REPO_PROGRAM='
def entries($kind): [$records[] | select(.kind == $kind) | {number, title, reason}] | sort_by(.number);
def numbers($kind): [$records[] | select(.kind == $kind) | .number] | sort;
entries("t3") as $t3 | entries("t4") as $t4 | entries("t5") as $t5
| {repo: $repo, head: $head, docs_dir: $docs, adrs_in_force: $force,
	no_impact_paths: (numbers("no_impact") | length), no_impact_adrs: numbers("no_impact"),
	no_timestamp: (numbers("no_timestamp") | length), no_timestamp_adrs: numbers("no_timestamp"),
	unresolved_adrs: (numbers("unresolved_adr") | length),
	unresolved: ([$records[] | select(.kind == "unresolved") | {number, token}] | sort_by([.number, .token])),
	t3: $t3, t4: $t4, t5: $t5, counts: {t3: ($t3 | length), t4: ($t4 | length), t5: ($t5 | length)}}'

readonly TOTAL_PROGRAM='
def shuffle($seed): . as $in | length as $n
	| reduce range($n - 1; 0; -1) as $i ({a: $in, s: (($seed % 2147483646) + 1)};
		.s = ((.s * 48271) % 2147483647)
		| (.s % ($i + 1)) as $j
		| .a |= (.[$i] as $x | .[$i] = .[$j] | .[$j] = $x))
	| .a;
def pool($kind): [.[] | .repo as $repo | .[$kind][] | {repo: $repo, number, title, reason}];
. as $repos
| {repos: $repos, total: {repos: ($repos | length), adrs_in_force: ($repos | map(.adrs_in_force) | add // 0),
	no_impact_paths: ($repos | map(.no_impact_paths) | add // 0), no_timestamp: ($repos | map(.no_timestamp) | add // 0),
	unresolved_adrs: ($repos | map(.unresolved_adrs) | add // 0),
	t3: ($repos | map(.counts.t3) | add // 0), t4: ($repos | map(.counts.t4) | add // 0), t5: ($repos | map(.counts.t5) | add // 0)}}
| if $n == null then . else . + {sample: ([("t3", "t4", "t5") as $k | {($k): ($repos | pool($k) | shuffle($seed) | .[:$n])}] | add)} end'

readonly MARKDOWN_PROGRAM='
def cell: gsub("\\|"; "\\|") | gsub("\n"; " ");
def table: . as $r | "## \($r.repo)", "",
	"head \($r.head), docs \($r.docs_dir), ADRs in force: \($r.adrs_in_force), without impact paths: \($r.no_impact_paths), without timestamp: \($r.no_timestamp), unresolved: \($r.unresolved_adrs)", "",
	"| type | ADR | title | reason |", "| --- | --- | --- | --- |",
	(("t3", "t4", "t5") as $k | $r[$k][] | "| \($k) | \(.number) | \(.title | cell) | \(.reason | cell) |"), "";
(.repos[] | table),
(.sample // empty | to_entries[] | .key as $k | .value[] | "sample \($k): \(.repo) \(.number) \(.title | cell): \(.reason | cell)"),
"total: repos \(.total.repos), ADRs in force \(.total.adrs_in_force), without impact paths \(.total.no_impact_paths), without timestamp \(.total.no_timestamp), unresolved \(.total.unresolved_adrs), t3 \(.total.t3), t4 \(.total.t4), t5 \(.total.t5)"'

die() {
	printf 'count: %s\n' "$*" >&2
	exit 2
}

frontmatter_value() { # frontmatter_value <file> <key>
	awk -v key="$2" '
		NR == 1 && $0 != "---" { exit }
		NR > 1 && $0 == "---" { exit }
		NR > 1 && index($0, key ":") == 1 {
			value = substr($0, length(key) + 2)
			gsub(/^[ \t]+|[ \t\r]+$/, "", value)
			gsub(/^["'\'']|["'\'']$/, "", value)
			print value
			exit
		}' "$1"
}

body_of() { # body_of <file>: everything after the frontmatter
	awk 'NR == 1 && $0 != "---" { body = 1 } body { print; next } NR > 1 && $0 == "---" { body = 1 }' "$1"
}

epoch_of() { # epoch_of <timestamp>: epoch seconds, or nothing when it does not parse
	jq -nr --arg s "$1" "$EPOCH_PROGRAM" 2>/dev/null
}

expand_braces() { # expand_braces <text>: one line per expansion of its brace groups
	local text="$1" pre group post option options
	if [[ "$text" =~ ^(.*)\{([^{}]*)\}(.*)$ ]]; then
		pre="${BASH_REMATCH[1]}"
		group="${BASH_REMATCH[2]}"
		post="${BASH_REMATCH[3]}"
		IFS=, read -ra options <<<"$group"
		for option in "${options[@]+"${options[@]}"}"; do
			option="${option#"${option%%[![:space:]]*}"}"
			option="${option%"${option##*[![:space:]]}"}"
			expand_braces "$pre$option$post"
		done
	else
		printf '%s\n' "$text"
	fi
}

impact_field() { # impact_field <body file>: the text of the Implementation impact field
	awk -v marker="$IMPACT_MARKER" '
		field {
			if ($0 ~ /^[ \t\r]*$/ || $0 ~ /^\*\*[^*]+:\*\*/) { field = 0; done = 1 } else { print; next }
		}
		!done && (at = index($0, marker)) { field = 1; print substr($0, at + length(marker)) }' "$1"
}

path_like() { # path_like <token>: a repository path, not a version, a URL or free text
	local token="$1"
	[[ -n "$token" && ! "$token" =~ [[:space:]] && "$token" != *://* && "$token" != *[{}]* ]] || return 1
	[[ ! "$token" =~ ^v?[0-9]+(\.[0-9]+)*$ ]] || return 1
	[[ "$token" == */* || "$token" =~ \.[A-Za-z0-9]+$ ]]
}

token_paths() { # token_paths <raw token>: its brace expansions that are paths, one per line
	local path
	while IFS= read -r path; do
		path_like "$path" && printf '%s\n' "$path"
	done < <(expand_braces "$1")
}

impact_tokens() { # impact_tokens <body file>: the raw backticked tokens of the impact field that name a path
	local token
	impact_field "$1" | grep -o '`[^`]*`' | tr -d '`' | sort -u | while IFS= read -r token; do
		[[ -n "$(token_paths "$token")" ]] && printf '%s\n' "$token"
	done
}

body_words() { # body_words <body file>: every word of a record, each backticked span whole and each plain word, with brace groups expanded, one per line; only an exact match to a wanted file names it
	awk '
		function emit(text,   n, parts, i) {
			n = split(text, parts, ",")
			for (i = 1; i <= n; i++) {
				sub(/[.,]+$/, "", parts[i])
				if (parts[i] != "" && parts[i] !~ /[{}]/ && !(parts[i] in seen)) { seen[parts[i]] = 1; print parts[i] }
			}
		}
		function expand(text,   at, len, pre, post, n, options, i) {
			if (match(text, /\{[^{}]*\}/)) {
				at = RSTART; len = RLENGTH
				pre = substr(text, 1, at - 1)
				post = substr(text, at + len)
				n = split(substr(text, at + 1, len - 2), options, ",")
				for (i = 1; i <= n; i++) expand(pre options[i] post)
			} else emit(text)
		}
		{
			line = $0
			while (match(line, /\{[^}]*, +/)) {
				segment = substr(line, RSTART, RLENGTH)
				sub(/, +$/, ",", segment)
				line = substr(line, 1, RSTART - 1) segment substr(line, RSTART + RLENGTH)
			}
			while (match(line, /`[^` ]+`/)) {
				word = substr(line, RSTART + 1, RLENGTH - 2)
				line = substr(line, 1, RSTART - 1) " " substr(line, RSTART + RLENGTH)
				sub(/^\.\//, "", word)
				expand(word)
				if (sub(/(:[0-9][0-9:-]*|#L[0-9].*|::.*|\(.*\))$/, "", word)) expand(word)
			}
			while (match(line, "[A-Za-z0-9_./*?{},-]+")) {
				word = substr(line, RSTART, RLENGTH)
				line = substr(line, RSTART + RLENGTH)
				sub(/[.,]+$/, "", word)
				sub(/^\.\//, "", word)
				expand(word)
			}
		}' "$1"
}

resolve_token() { # resolve_token <repo> <head files> <token>: the files it names at HEAD, or nonzero when none does
	local repo="$1" files="$2" token="${3#./}" matches plain=1
	[[ "$3" == */ ]] && plain=0
	token="${token%/}"
	[[ -n "$token" ]] || return 1
	if [[ "$token" == *[*?]* ]]; then
		plain=0
		matches="$(git -c core.quotepath=off -C "$repo" ls-files -- ":(glob)$token" 2>/dev/null | sort)"
	else
		matches="$(awk -v path="$token" '$0 == path || index($0, path "/") == 1' "$files")"
		if [[ -z "$matches" && $plain == 1 && -n "$(git --literal-pathspecs -C "$repo" log --all --format=%H -1 -- "$token" 2>/dev/null)" ]]; then
			matches="$token"
		fi
	fi
	if [[ -z "$matches" && $plain == 1 ]]; then
		matches="$(awk -v suffix="/$token" 'length($0) > length(suffix) && substr($0, length($0) - length(suffix) + 1) == suffix' "$files")"
		[[ "$(grep -c . <<<"$matches")" == 1 ]] || return 1
	fi
	[[ -n "$matches" ]] || return 1
	printf '%s\n' "$matches"
}

rec() { # rec <kind> <number> [title] [reason] [token]
	jq -nc --arg kind "$1" --argjson number "$2" --arg title "${3:-}" --arg reason "${4:-}" --arg token "${5:-}" \
		'{kind: $kind, number: $number, title: $title, reason: $reason, token: $token}'
}

stale_files() { # stale_files <number> <title> <timestamp> <counts file>: the t3 or t4 record, if any
	local number="$1" title="$2" stamp="$3" count named file any=0 flagged=""
	while IFS=$'\t' read -r count named file; do
		((count > 0)) && any=1
		if [[ -z "$flagged" ]] && ((count >= MIN_COMMITS)) && ((named == 0)); then
			flagged="$file changed in $count commits after $stamp"
		fi
	done <"$4"
	if [[ -n "$flagged" ]]; then
		rec t3 "$number" "$title" "$flagged"
	elif ((any == 0)); then
		rec t4 "$number" "$title" "no impact path changed after $stamp"
	fi
}

stage_named() { # stage_named <repo> <docs path> <scratch>: named.max, "path<TAB>latest instant" of every path a dated record names, deleted files included
	local file epoch token path resolved
	: >"$3/words.tsv"
	: >"$3/paths.tsv"
	while IFS= read -r file; do
		epoch="$(epoch_of "$(frontmatter_value "$file" timestamp)")"
		[[ -n "$epoch" ]] || continue
		body_of "$file" >"$3/rec.body"
		body_words "$3/rec.body" | awk -v epoch="$epoch" '{ print epoch "\t" $0 }' >>"$3/words.tsv"
		grep -qF "$IMPACT_MARKER" "$3/rec.body" || continue
		while IFS= read -r token; do
			token_paths "$token" | awk -v epoch="$epoch" '{ print epoch "\t" $0 }' >>"$3/paths.tsv"
		done < <(impact_tokens "$3/rec.body")
	done < <(find "$2" -type f -name '*.md' | sort)
	: >"$3/resolved.tsv"
	while IFS= read -r path; do
		if resolved="$(resolve_token "$1" "$3/headfiles" "$path")"; then
			awk -v path="$path" '{ print path "\t" $0 }' <<<"$resolved" >>"$3/resolved.tsv"
		fi
	done < <(cut -f2 "$3/paths.tsv" | sort -u)
	awk -F'\t' '
		function name(file, epoch) { if (!(file in latest) || epoch > latest[file]) latest[file] = epoch }
		FILENAME == resolved { count[$1]++; target[$1, count[$1]] = $2; next }
		FILENAME == words { name($2, $1); next }
		{ for (i = 1; i <= count[$2]; i++) name(target[$2, i], $1) }
		END { for (f in latest) print f "\t" latest[f] }' resolved="$3/resolved.tsv" words="$3/words.tsv" \
		"$3/resolved.tsv" "$3/words.tsv" "$3/paths.tsv" | sort >"$3/named.max"
}

commit_table() { # commit_table <repo>: "committer epoch<TAB>file" for every file of every non-merge commit reachable from HEAD
	git -c core.quotepath=off -C "$1" log --no-merges --no-renames --name-only --format='@%ct' |
		awk '/^@[0-9]+$/ { stamp = substr($0, 2); next } NF { print stamp "\t" $0 }'
}

file_counts() { # file_counts <commit table> <files file> <epoch> <named max file>: "commits after<TAB>named after<TAB>file" per listed file
	awk -F'\t' -v after="$3" -v list="$2" -v latest="$4" '
		FILENAME == list { want[$0] = 1; next }
		FILENAME == latest { named[$1] = $2; next }
		$1 > after && ($2 in want) { seen[$2]++ }
		END { for (f in want) print seen[f] + 0 "\t" ((f in named) && named[f] > after ? 1 : 0) "\t" f }' "$2" "$4" "$1" | sort -t$'\t' -k3
}

resolve_tokens() { # resolve_tokens <repo> <head files> <tokens file> <out file> [<number> <records file>]: appends the files the tokens resolve to; with a number, lists each token that fails
	local token path resolved
	while IFS= read -r token; do
		while IFS= read -r path; do
			if resolved="$(resolve_token "$1" "$2" "$path")"; then
				printf '%s\n' "$resolved" >>"$4"
			elif [[ -n "${5:-}" ]]; then
				rec unresolved "$5" "" "" "$path" >>"$6"
			fi
		done < <(token_paths "$token")
	done <"$3"
}

resolve_adr() { # resolve_adr <repo> <head files> <tokens file> <number> <scratch>: fills adr.files and appends unresolved records
	: >"$5/adr.files"
	resolve_tokens "$1" "$2" "$3" "$5/adr.files" "$4" "$5/records.jsonl"
	sort -u -o "$5/adr.files" "$5/adr.files"
}

count_adr() { # count_adr <repo> <scratch> <number> <title> <timestamp> <epoch>: one ADR's records, appended to records.jsonl
	local repo="$1" scratch="$2" number="$3" title="$4" stamp="$5" epoch="$6" values
	impact_tokens "$scratch/adr.body" >"$scratch/adr.tokens"
	if [[ ! -s "$scratch/adr.tokens" ]]; then
		rec no_impact "$number" >>"$scratch/records.jsonl"
	else
		resolve_adr "$repo" "$scratch/headfiles" "$scratch/adr.tokens" "$number" "$scratch"
		if [[ ! -s "$scratch/adr.files" ]]; then
			rec unresolved_adr "$number" >>"$scratch/records.jsonl"
		else
			file_counts "$scratch/commits.tsv" "$scratch/adr.files" "$epoch" "$scratch/named.max" >"$scratch/adr.counts"
			stale_files "$number" "$title" "$stamp" "$scratch/adr.counts" >>"$scratch/records.jsonl"
		fi
	fi
	values="$(jq -nr --rawfile body "$scratch/adr.body" --rawfile tokens "$scratch/adr.tokens" "$VALUES_PROGRAM | join(\"; \")")"
	[[ -n "$values" ]] && rec t5 "$number" "$title" "names $values" >>"$scratch/records.jsonl"
}

count_repo() { # count_repo <repo> <docs dir name> <scratch dir>
	local repo="$1" docs="$2" scratch="$3" head file name number title stamp epoch force=0
	git -C "$repo" rev-parse --git-dir >/dev/null 2>&1 || die "not a git repository: $repo"
	head="$(git -C "$repo" rev-parse HEAD 2>/dev/null)" || die "no commits in repository: $repo"
	[[ -d "$repo/$docs/adr" ]] || die "no $docs/adr directory in $repo"
	: >"$scratch/records.jsonl"
	git -c core.quotepath=off -C "$repo" ls-files | sort >"$scratch/headfiles"
	commit_table "$repo" >"$scratch/commits.tsv"
	stage_named "$repo" "$repo/$docs" "$scratch"
	while IFS= read -r file; do
		name="$(basename "$file")"
		[[ "$name" =~ ^([0-9]{4})-.*\.md$ ]] || continue
		number=$((10#${BASH_REMATCH[1]}))
		[[ "$(frontmatter_value "$file" type)" == ADR && "$(frontmatter_value "$file" status)" == Accepted ]] || continue
		force=$((force + 1))
		title="$(frontmatter_value "$file" title)"
		stamp="$(frontmatter_value "$file" timestamp)"
		epoch="$(epoch_of "$stamp")"
		if [[ -z "$epoch" ]]; then
			rec no_timestamp "$number" >>"$scratch/records.jsonl"
			continue
		fi
		body_of "$file" >"$scratch/adr.body"
		count_adr "$repo" "$scratch" "$number" "$title" "$stamp" "$epoch"
	done < <(find "$repo/$docs/adr" -maxdepth 1 -type f -name '*.md' | sort)
	jq -n --arg repo "$repo" --arg head "$head" --arg docs "$docs" --argjson force "$force" \
		--slurpfile records "$scratch/records.jsonl" "$REPO_PROGRAM"
}

main() {
	local markdown=0 docs=docs repos=() results=() result repo json sample=null seed=0
	command -v jq >/dev/null 2>&1 || die "jq not found on PATH"
	command -v git >/dev/null 2>&1 || die "git not found on PATH"
	while (($# > 0)); do
		case "$1" in
		--markdown) markdown=1 ;;
		--docs-dir)
			(($# >= 2)) || die "--docs-dir needs a name"
			docs="$2"
			shift
			;;
		--sample)
			(($# >= 2)) && [[ "$2" =~ ^[0-9]+$ ]] || die "--sample needs a whole number"
			sample="$2"
			shift
			;;
		--seed)
			(($# >= 2)) && [[ "$2" =~ ^[0-9]+$ ]] || die "--seed needs a whole number"
			seed="$2"
			shift
			;;
		-h | --help) sed -n '3,39p' "$0" && exit 0 ;;
		-*) die "unknown option: $1" ;;
		*) repos+=("$1") ;;
		esac
		shift
	done
	((${#repos[@]} > 0)) || die "usage: count.sh [--markdown] [--docs-dir NAME] [--sample N] [--seed S] REPO..."
	scratch="$(mktemp -d "${TMPDIR:-/tmp}/drift-count.XXXXXX")" || die "cannot make a scratch directory"
	trap 'rm -rf "$scratch"' EXIT
	for repo in "${repos[@]}"; do
		result="$(count_repo "$repo" "$docs" "$scratch")" || exit $?
		results+=("$result")
	done
	json="$(printf '%s\n' "${results[@]}" | jq -s --argjson n "$sample" --argjson seed "$seed" "$TOTAL_PROGRAM")"
	if ((markdown)); then jq -r "$MARKDOWN_PROGRAM" <<<"$json"; else printf '%s\n' "$json"; fi
}

main "$@"
