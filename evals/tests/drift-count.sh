#!/usr/bin/env bash
#
# drift-count.sh — tests for evals/drift-count/count.sh over fixture git repositories with
# explicit commit dates. Sourced by run.sh, which provides TMP, EVALS, assert_eq, assert_true and
# wanted. Fixtures live under $TMP and no clock, network or model is involved.

COUNT="$EVALS/drift-count/count.sh"
FX_DOCS=docs
FX_REFERENCE="ADR ""0012"
FX_TICKET="issue ""0153"

fx_git() { # fx_git <repo> <date> <git args...>
	local repo="$1" date="$2"
	shift 2
	GIT_AUTHOR_DATE="$date" GIT_COMMITTER_DATE="$date" GIT_AUTHOR_NAME=fx GIT_AUTHOR_EMAIL=fx@example.invalid \
		GIT_COMMITTER_NAME=fx GIT_COMMITTER_EMAIL=fx@example.invalid \
		git -C "$repo" -c commit.gpgsign=false -c core.hooksPath=/dev/null -c init.defaultBranch=main "$@" >/dev/null
}

fx_repo() { # fx_repo <name>: prints the new repository path
	local repo="$TMP/drift-$1"
	rm -rf "$repo"
	mkdir -p "$repo/docs/adr"
	fx_git "$repo" "2026-01-01T00:00:00Z" init
	printf '%s' "$repo"
}

fx_commit() { # fx_commit <repo> <date> <file>...: one commit appending a line to each file
	local repo="$1" date="$2" file
	shift 2
	for file in "$@"; do
		mkdir -p "$repo/$(dirname "$file")"
		printf '%s %s\n' "$date" "$file" >>"$repo/$file"
	done
	fx_git "$repo" "$date" add -A
	fx_git "$repo" "$date" commit -m "change $date"
}

fx_record() { # fx_record <repo> <relative path> <type> <status> <timestamp> <title> <body>
	local repo="$1" rel="$2"
	mkdir -p "$repo/$(dirname "$rel")"
	printf -- '---\ntype: %s\ntitle: %s\nstatus: %s\ntimestamp: %s\n---\n\n# %s\n\n%s\n' \
		"$3" "$6" "$4" "$5" "$6" "$7" >"$repo/$rel"
}

fx_adr_at() { # fx_adr_at <repo> <number> <status> <timestamp> <title> <impact field or empty> <extra body>
	local impact="" body="$7"
	[[ -n "$6" ]] && impact=$'**Implementation impact:** '"$6"$'\n\n'
	fx_record "$1" "docs/adr/$(printf '%04d' "$2")-fixture.md" ADR "$3" "$4" "$5" "${impact}${body}"
}

fx_adr() { # fx_adr <repo> <number> <status> <title> <impact field or empty> <extra body>
	fx_adr_at "$1" "$2" "$3" "2026-02-01T00:00:00Z" "$4" "$5" "$6"
}

fx_seal() { # fx_seal <repo>: commits the records written so far, before every ADR date
	fx_git "$1" "2026-01-01T00:00:00Z" add -A
	fx_git "$1" "2026-01-01T00:00:00Z" commit -m docs
}

drift_rows() { # drift_rows <repo> <jq path>: compact rows of one repository's result
	bash "$COUNT" "$1" | jq -c ".repos[0] | $2"
}

fx_build_changes() { # a repository with boundary cases for the changed-path rule
	local r
	r="$(fx_repo changes)"
	fx_adr "$r" 1 Accepted "Three commits" '`src/a.txt`' "Plain."
	fx_adr "$r" 2 Accepted "Two commits" '`src/b.txt`' "Plain."
	fx_adr "$r" 3 Accepted "Three commits but named later" '`src/c.txt`' "Plain."
	fx_adr "$r" 4 Accepted "Three commits all before" '`src/d.txt`' "Plain."
	fx_adr "$r" 5 Accepted "Two commits and a merge" '`src/m.txt`' "Plain."
	fx_adr "$r" 6 Accepted "Directory path" '`lib/`' "Plain."
	fx_adr "$r" 7 Accepted "Directory spread over files" '`other/`' "Plain."
	fx_record "$r" "$FX_DOCS/issues/0001-later.md" Issue Open "2026-04-01T00:00:00Z" "A later record" 'It touches `src/c.txt` again.'
	fx_record "$r" "$FX_DOCS/issues/0002-earlier.md" Issue Open "2026-01-15T00:00:00Z" "An earlier record" 'It touches `src/a.txt` before.'
	fx_git "$r" "2026-01-01T00:00:00Z" add -A
	fx_git "$r" "2026-01-01T00:00:00Z" commit -m docs
	fx_commit "$r" "2026-01-05T00:00:00Z" src/d.txt
	fx_commit "$r" "2026-01-06T00:00:00Z" src/d.txt
	fx_commit "$r" "2026-01-07T00:00:00Z" src/d.txt
	fx_commit "$r" "2026-03-01T00:00:00Z" src/a.txt src/b.txt src/c.txt src/m.txt lib/x.txt other/1.txt
	fx_commit "$r" "2026-03-02T00:00:00Z" src/a.txt src/b.txt src/c.txt lib/x.txt lib/y.txt other/2.txt
	fx_git "$r" "2026-03-02T12:00:00Z" checkout -b side
	fx_commit "$r" "2026-03-03T00:00:00Z" src/m.txt
	fx_git "$r" "2026-03-03T06:00:00Z" checkout main
	fx_commit "$r" "2026-03-04T00:00:00Z" src/a.txt src/c.txt lib/x.txt lib/z.txt other/3.txt
	fx_git "$r" "2026-03-06T00:00:00Z" merge --no-ff --no-commit side
	printf 'merge resolution\n' >>"$r/src/m.txt"
	fx_git "$r" "2026-03-06T00:00:00Z" add -A
	fx_git "$r" "2026-03-06T00:00:00Z" commit -m merge
	printf '%s' "$r"
}

fx_build_force() { # a repository with every status, a non-record and the T4 cases
	local r
	r="$(fx_repo force)"
	fx_adr "$r" 1 Accepted "Never touched" '`ghost/never.txt`' "Plain."
	fx_adr "$r" 2 Accepted "One path touched" '`x/one.txt` and `x/two.txt`' "Plain."
	fx_adr "$r" 3 Proposed "Proposed" '`ghost/p.txt`' "Plain."
	fx_adr "$r" 4 Superseded "Superseded" '`ghost/s.txt`' "Plain."
	fx_adr "$r" 5 Deprecated "Deprecated" '`ghost/d.txt`' "Plain."
	fx_adr "$r" 6 Accepted "No impact line" "" "Plain."
	fx_adr "$r" 7 Accepted "No path token" '`crate` and `Config`' "Plain."
	fx_record "$r" "$FX_DOCS/adr/0009-notes.md" Note Accepted "2026-02-01T00:00:00Z" "Not an ADR" '**Implementation impact:** `ghost/n.txt`'
	fx_record "$r" "$FX_DOCS/issues/0010-issue.md" Issue Accepted "2026-02-01T00:00:00Z" "An issue" '**Implementation impact:** `ghost/i.txt`'
	fx_git "$r" "2026-01-01T00:00:00Z" add -A
	fx_git "$r" "2026-01-01T00:00:00Z" commit -m docs
	fx_commit "$r" "2026-03-01T00:00:00Z" x/two.txt
	printf '%s' "$r"
}

fx_build_values() { # a repository whose ADRs name values or only look like them
	local r
	r="$(fx_repo values)"
	fx_adr "$r" 1 Accepted "Database" "" "We run PostgreSQL 15 in production."
	fx_adr "$r" 2 Accepted "Release" "" "The format ships as v2.3.1 first."
	fx_adr "$r" 3 Accepted "Price" "" 'The hosted plan costs $20 per month.'
	fx_adr "$r" 4 Accepted "Size" "" "Uploads stop at 100 MB each."
	fx_adr "$r" 5 Accepted "Limit" "" "The vendor sets a rate limit of 50 req/s."
	fx_adr "$r" 6 Accepted "Numbers" "" "See $FX_REFERENCE, decided 2026-11-24, with three services and 12 tests."
	fx_git "$r" "2026-01-01T00:00:00Z" add -A
	fx_git "$r" "2026-01-01T00:00:00Z" commit -m docs
	printf '%s' "$r"
}

test_drift_count_in_force() {
	echo "drift count: in force"
	local r
	r="$(fx_build_force)"
	assert_eq "only Accepted ADRs are in force" 4 "$(drift_rows "$r" .adrs_in_force)"
	assert_eq "an ADR with no impact line or no path token is counted without a path" "[6,7]" "$(drift_rows "$r" .no_impact_adrs)"
	assert_eq "no_impact_paths counts them" 2 "$(drift_rows "$r" .no_impact_paths)"
	assert_eq "they appear in neither t3 nor t4, nor does an ADR whose only path never existed" "[]" "$(drift_rows "$r" '[.t3[], .t4[]] | map(.number)')"
}

test_drift_count_changed() {
	echo "drift count: changed path"
	local r
	r="$(fx_build_changes)"
	assert_eq "three commits after the date with no newer record is a candidate" "[1,6]" "$(drift_rows "$r" '.t3 | map(.number)')"
	assert_eq "the reason names the path and the commit count" '"src/a.txt changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[0].reason')"
	assert_eq "a directory path applies the rule to each file under it" '"lib/x.txt changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[1].reason')"
	assert_eq "a directory whose commits are spread one per file is neither changed nor untouched" "[]" \
		"$(drift_rows "$r" '[.t3[], .t4[]] | map(.number) | map(select(. == 7))')"
	assert_eq "only the ADR whose commits all precede its date has an untouched path" "[4]" "$(drift_rows "$r" '.t4 | map(.number)')"
}

test_drift_count_untouched() {
	echo "drift count: untouched path"
	local r
	r="$(fx_build_force)"
	local c
	c="$(fx_build_changes)"
	assert_eq "a path that never existed is unresolved, not untouched" "[]" "$(drift_rows "$r" '.t4 | map(.number)')"
	assert_eq "the reason of an untouched path names the date" '"no impact path changed after 2026-02-01T00:00:00Z"' "$(drift_rows "$c" '.t4[0].reason')"
}

test_drift_count_values() {
	echo "drift count: values"
	local r
	r="$(fx_build_values)"
	assert_eq "values name version, price, limit and quota, not numbers or dates" "[1,2,3,4,5]" "$(drift_rows "$r" '.t5 | map(.number)')"
	assert_true "the version product match is recorded" grep -q 'PostgreSQL 15' <<<"$(drift_rows "$r" '.t5[0].reason')"
	assert_true "the version token match is recorded" grep -q 'v2.3.1' <<<"$(drift_rows "$r" '.t5[1].reason')"
	assert_true "the price match is recorded" grep -q '\$20 per month' <<<"$(drift_rows "$r" '.t5[2].reason')"
	assert_true "the size match is recorded" grep -q '100 MB' <<<"$(drift_rows "$r" '.t5[3].reason')"
	assert_true "the rate match is recorded" grep -q '50 req/s' <<<"$(drift_rows "$r" '.t5[4].reason')"
}

fx_build_field() { # impact fields that span lines, bullets, brace groups and version tokens
	local r d
	r="$(fx_repo field)"
	fx_adr "$r" 1 Accepted "Continuation line" $'`c/f1.txt`\nand also `c/deep.txt`' "Plain."
	fx_adr "$r" 2 Accepted "Brace group" '`c/{f1.txt, b.rs}`' "Plain."
	fx_adr "$r" 3 Accepted "Bullet list" $'\n- `c/f1.txt`\n- `c/bullet.txt`' "Plain."
	fx_adr "$r" 4 Accepted "Next field marker" $'`c/f1.txt`\n**Consequences:** `c/f2.txt`' "Plain."
	fx_adr "$r" 5 Accepted "Blank line" '`c/f1.txt`' 'Elsewhere `c/f2.txt` is named.'
	fx_adr "$r" 6 Accepted "Versions only" '`0.34.0` and `v2`' "Plain."
	fx_seal "$r"
	fx_commit "$r" "2026-03-01T00:00:00Z" c/f1.txt c/f2.txt c/b.rs c/deep.txt c/bullet.txt
	for d in 2026-03-02T00:00:00Z 2026-03-03T00:00:00Z; do
		fx_commit "$r" "$d" c/f2.txt c/b.rs c/deep.txt c/bullet.txt
	done
	printf '%s' "$r"
}

fx_build_resolve() { # tokens that name directories, globs, crate-relative files, deleted files or nothing
	local r
	r="$(fx_repo resolve)"
	fx_adr "$r" 1 Accepted "Directory" '`dir/`' "Plain."
	fx_adr "$r" 2 Accepted "Glob" '`g/*.log`' "Plain."
	fx_adr "$r" 3 Accepted "Crate-relative path" '`commands/effective.rs`' "Plain."
	fx_adr "$r" 4 Accepted "Bare file name" '`effective.rs`' "Plain."
	fx_adr "$r" 5 Accepted "Deleted file" '`old/gone.txt`' "Plain."
	fx_adr "$r" 6 Accepted "Nothing matches" '`x/nothing.rs`' "Plain."
	fx_adr "$r" 7 Accepted "Two suffix matches" '`dup.rs`' "Plain."
	fx_adr "$r" 8 Accepted "One resolved and one not" '`dir/q.txt` and `zz/missing.txt`' "Plain."
	fx_adr "$r" 9 Accepted "Spread over a directory" '`s/`' "Plain."
	fx_seal "$r"
	fx_commit "$r" "2026-03-01T00:00:00Z" dir/p.txt dir/q.txt g/a.log g/b.log crate/src/commands/effective.rs x/dup.rs y/dup.rs old/gone.txt s/1.txt
	fx_commit "$r" "2026-03-02T00:00:00Z" dir/p.txt g/a.log crate/src/commands/effective.rs old/gone.txt s/2.txt
	rm "$r/old/gone.txt"
	fx_commit "$r" "2026-03-03T00:00:00Z" dir/p.txt g/a.log crate/src/commands/effective.rs s/3.txt
	printf '%s' "$r"
}

fx_build_instants() { # commit dates at the exact second of an ADR, with an offset form and a skewed history
	local r
	r="$(fx_repo instants)"
	fx_adr_at "$r" 1 Accepted "2026-02-01T12:00:00Z" "Exact second" '`t/exact.txt`' "Plain."
	fx_adr_at "$r" 2 Accepted "2026-02-01T12:00:00Z" "Skewed history" '`t/skew.txt`' "Plain."
	fx_adr_at "$r" 3 Accepted "2026-02-01T14:00:00+02:00" "Offset form" '`t/off.txt`' "Plain."
	fx_seal "$r"
	fx_commit "$r" "2026-03-01T00:00:00Z" t/skew.txt
	fx_commit "$r" "2026-01-01T00:00:00Z" t/skew.txt
	fx_commit "$r" "2026-03-03T00:00:00Z" t/skew.txt
	fx_commit "$r" "2026-03-04T00:00:00Z" t/skew.txt
	fx_commit "$r" "2026-02-01T11:59:59Z" t/exact.txt t/off.txt
	fx_commit "$r" "2026-02-01T12:00:00Z" t/exact.txt t/off.txt
	fx_commit "$r" "2026-02-01T12:00:01Z" t/exact.txt t/off.txt
	fx_commit "$r" "2026-02-01T12:00:02Z" t/exact.txt t/off.txt
	fx_commit "$r" "2026-02-01T12:00:03Z" t/exact.txt t/off.txt
	printf '%s' "$r"
}

fx_build_stamps() { # missing, unparseable and date-only timestamps, and newer records in another offset
	local r
	r="$(fx_repo stamps)"
	fx_adr_at "$r" 1 Accepted "" "No timestamp" '`n/nots.txt`' "Runs on Node 20."
	fx_adr_at "$r" 2 Accepted "soon" "Unparseable timestamp" '`n/nots.txt`' "Runs on Node 20."
	fx_adr_at "$r" 3 Accepted "2026-03-05" "Date only" '`d/day.txt`' "Plain."
	fx_adr_at "$r" 4 Accepted "2026-02-01T09:00:00Z" "Older record in another offset" '`n/inst.txt`' "Plain."
	fx_adr_at "$r" 5 Accepted "2026-02-01T09:00:00Z" "Newer record in another offset" '`n/inst2.txt`' "Plain."
	fx_record "$r" "$FX_DOCS/issues/0001-older.md" Issue Open "2026-02-01T10:00:00+02:00" "Older by instant" 'It names `n/inst.txt`.'
	fx_record "$r" "$FX_DOCS/issues/0002-newer.md" Issue Open "2026-02-01T08:30:00-02:00" "Newer by instant" 'It names `n/inst2.txt`.'
	fx_seal "$r"
	fx_commit "$r" "2026-03-01T00:00:00Z" n/nots.txt n/inst.txt n/inst2.txt
	fx_commit "$r" "2026-03-02T00:00:00Z" n/nots.txt n/inst.txt n/inst2.txt
	fx_commit "$r" "2026-03-03T00:00:00Z" n/nots.txt n/inst.txt n/inst2.txt
	fx_commit "$r" "2026-03-04T23:59:59Z" d/day.txt
	fx_commit "$r" "2026-03-05T00:00:00Z" d/day.txt
	fx_commit "$r" "2026-03-05T00:00:01Z" d/day.txt
	fx_commit "$r" "2026-03-05T00:00:02Z" d/day.txt
	fx_commit "$r" "2026-03-05T00:00:03Z" d/day.txt
	printf '%s' "$r"
}

fx_build_naming() { # newer records that name a file in prose, or a directory, glob or crate-relative path in their impact field
	local r d
	r="$(fx_repo naming)"
	fx_adr "$r" 1 Accepted "Look-alike names" '`src/a.rs`' "Plain."
	fx_adr "$r" 2 Accepted "Named by path in prose" '`src/b.rs`' "Plain."
	fx_adr "$r" 3 Accepted "Directory in prose only" '`lib/n.rs`' "Plain."
	fx_adr "$r" 4 Accepted "Glob in prose only" '`glob/z.log`' "Plain."
	fx_adr "$r" 5 Accepted "Directory in impact field" '`dirimp/f.rs`' "Plain."
	fx_adr "$r" 6 Accepted "Glob in impact field" '`globimp/g.log`' "Plain."
	fx_adr "$r" 7 Accepted "Crate-relative token in impact field" '`crate/src/commands/effective.rs`' "Plain."
	fx_adr "$r" 8 Accepted "Ambiguous suffix in impact field" '`x/dup.rs`' "Plain."
	fx_adr "$r" 9 Accepted "Named only by older or same-instant records" '`old/o.rs`' "Plain."
	fx_record "$r" "$FX_DOCS/issues/0001-lookalikes.md" Issue Open "2026-04-01T00:00:00Z" "Look-alikes" \
		$'**Implementation impact:** `src/a` and `rc/`\n\nIt names `mysrc/a.rs`, `src/a.rsx`, `srcx/`, src/a and rc/.'
	fx_record "$r" "$FX_DOCS/issues/0002-path.md" Issue Open "2026-04-01T00:00:00Z" "Path" 'It names `src/b.rs`.'
	fx_record "$r" "$FX_DOCS/issues/0003-dir.md" Issue Open "2026-04-01T00:00:00Z" "Directory" 'It touches lib/ broadly and rewrites `glob/*.log` files.'
	fx_record "$r" "$FX_DOCS/issues/0004-impact.md" Issue Open "2026-04-01T00:00:00Z" "Impact field" \
		$'**Implementation impact:** `dirimp/`, `globimp/*.log`, `commands/effective.rs` and `dup.rs`\n\nPlain.'
	fx_record "$r" "$FX_DOCS/issues/0005-same.md" Issue Open "2026-02-01T00:00:00Z" "Same instant" 'It names `old/o.rs`.'
	fx_record "$r" "$FX_DOCS/issues/0006-older.md" Issue Open "2026-01-15T00:00:00Z" "Older" $'**Implementation impact:** `old/`\n\nIt names `old/o.rs`.'
	fx_seal "$r"
	for d in 2026-03-01T00:00:00Z 2026-03-02T00:00:00Z 2026-03-03T00:00:00Z; do
		fx_commit "$r" "$d" src/a.rs src/b.rs lib/n.rs glob/z.log dirimp/f.rs globimp/g.log crate/src/commands/effective.rs x/dup.rs y/dup.rs old/o.rs
	done
	printf '%s' "$r"
}

fx_build_naming_kept() { # deleted files and paths with + or @, each named by a newer record, plus one look-alike left unnamed
	local r d
	r="$(fx_repo naming-kept)"
	fx_adr "$r" 1 Accepted "Deleted file named in prose" '`old/gone.rs`' "Plain."
	fx_adr "$r" 2 Accepted "Deleted file named in an impact field" '`old/gone2.rs`' "Plain."
	fx_adr "$r" 3 Accepted "Route file with a plus" '`src/routes/+page.svelte`' "Plain."
	fx_adr "$r" 4 Accepted "Scoped package file" '`packages/@x/y.ts`' "Plain."
	fx_adr "$r" 5 Accepted "Scoped package file left unnamed" '`packages/@x/z.ts`' "Plain."
	fx_adr "$r" 6 Accepted "File named with a line" '`src/l.rs`' "Plain."
	fx_adr "$r" 7 Accepted "File named with an anchor" '`src/m.rs`' "Plain."
	fx_adr "$r" 8 Accepted "File named with an item" '`crate/src/c.rs`' "Plain."
	fx_adr "$r" 9 Accepted "File named with a call" '`src/d.rs`' "Plain."
	fx_adr "$r" 10 Accepted "File mentioned only with a non-line suffix" '`src/n.rs`' "Plain."
	fx_adr "$r" 11 Accepted "File named with a line and column" '`src/k.rs`' "Plain."
	fx_adr "$r" 12 Accepted "Root file without an extension" '`./*`' "Plain."
	fx_adr "$r" 13 Accepted "Root file named as a bare word" '`./Dockerfile`' "Plain."
	fx_record "$r" "$FX_DOCS/issues/0005-bare.md" Issue Open "2026-04-01T00:00:00Z" "Bare" 'It rebuilds Dockerfile for arm64 hosts'
	fx_record "$r" "$FX_DOCS/issues/0004-suffixes.md" Issue Open "2026-04-01T00:00:00Z" "Suffixes" \
		'Fix at `src/l.rs:42`, `src/m.rs#L10`, `crate/src/c.rs::run`, `src/d.rs(12)`, `src/k.rs:42:7` and `Makefile:12`; see `src/n.rs:abc`, `src/n.rs:42abc` and `src/n.rs#top`.'
	fx_record "$r" "$FX_DOCS/issues/0001-retire.md" Issue Open "2026-04-01T00:00:00Z" "Retire" 'It retires `old/gone.rs`.'
	fx_record "$r" "$FX_DOCS/issues/0002-retire.md" Issue Open "2026-04-01T00:00:00Z" "Retire again" $'**Implementation impact:** `old/gone2.rs`\n\nPlain.'
	fx_record "$r" "$FX_DOCS/issues/0003-symbols.md" Issue Open "2026-04-01T00:00:00Z" "Symbols" 'It rewrites `src/routes/+page.svelte` and `packages/@x/y.ts`.'
	fx_seal "$r"
	for d in 2026-03-01T00:00:00Z 2026-03-02T00:00:00Z; do
		fx_commit "$r" "$d" old/gone.rs old/gone2.rs src/routes/+page.svelte packages/@x/y.ts packages/@x/z.ts src/l.rs src/m.rs crate/src/c.rs src/d.rs src/n.rs src/k.rs Makefile Dockerfile
	done
	fx_git "$r" 2026-03-03T00:00:00Z rm -q old/gone.rs old/gone2.rs
	fx_commit "$r" 2026-03-03T00:00:00Z src/routes/+page.svelte packages/@x/y.ts packages/@x/z.ts src/l.rs src/m.rs crate/src/c.rs src/d.rs src/n.rs src/k.rs Makefile Dockerfile
	printf '%s' "$r"
}

FX_GLOB_FILES=(cli/src/m.rs cli/src/sub/n.rs cli/src/sub/deep/o.rs a1.md ab.md abc.md a/b.md)

fx_build_glob_naming() { # fx_build_glob_naming <name> <glob>: one ADR per file, one newer record whose impact field holds the glob
	local r i=0 file d
	r="$(fx_repo "$1")"
	for file in "${FX_GLOB_FILES[@]}"; do
		i=$((i + 1))
		fx_adr "$r" "$i" Accepted "File $file" "\`$file\`" "Plain."
	done
	fx_record "$r" "$FX_DOCS/issues/0001-glob.md" Issue Open "2026-04-01T00:00:00Z" "Glob" $'**Implementation impact:** `'"$2"$'`\n\nPlain.'
	fx_seal "$r"
	for d in 2026-03-01T00:00:00Z 2026-03-02T00:00:00Z 2026-03-03T00:00:00Z; do
		fx_commit "$r" "$d" "${FX_GLOB_FILES[@]}"
	done
	printf '%s' "$r"
}

fx_build_glob_adr() { # fx_build_glob_adr <name> <glob> <file>...: one ADR whose impact glob resolves against the fixture files, of which only those named change after it
	local r name="$1" glob="$2" d file
	shift 2
	r="$(fx_repo "$name")"
	fx_adr "$r" 1 Accepted "Glob" "\`$glob\`" "Plain."
	fx_seal "$r"
	for file in "${FX_GLOB_FILES[@]}"; do fx_commit "$r" "2026-01-02T00:00:00Z" "$file"; done
	for d in 2026-03-01T00:00:00Z 2026-03-02T00:00:00Z 2026-03-03T00:00:00Z; do
		fx_commit "$r" "$d" "$@"
	done
	printf '%s' "$r"
}

fx_build_patterns() { # one ADR per phrase: numbers 1 to 11 name a value, the rest only look like one
	local r i=0 phrase
	r="$(fx_repo patterns)"
	for phrase in 'Runs on Node 20.' 'Needs Java 21 here.' 'Targets Python 3.11.' 'Uses PostgreSQL 15.' 'Ships as v2.3.1 first.' 'Run `postgres:15` locally.' \
		'The plan costs $20 per month.' 'About 1000 per month are sent.' 'There is a quota of 500.' 'The vendor sets a rate limit of 50 req/s.' 'Uploads stop at 100 MB each.' 'Pins rmcp 3.9.0.' 'Depends on rmcp (3.9.0 on crates.io).'; do
		i=$((i + 1))
		fx_adr "$r" "$i" Accepted "Names a value" "" "$phrase"
	done
	for phrase in "See $FX_REFERENCE." "See $FX_TICKET." 'Stage 2 starts.' 'See section 4.2.' 'See step 3.1.' 'Uptime is 99.9%.' 'It took 1.19 s.' 'It took 250 ms.' \
		'It binds 127.0.0.1 only.' 'It maps 1.2.3.4 to a host.' 'Of 41 cases, 3 failed.' 'About 97 are cached.' 'Exit 0 means success.' 'Pass `$1` to the script.' 'Decided 2026-11-24.' 'We run 12 tests.'; do
		i=$((i + 1))
		fx_adr "$r" "$i" Accepted "Only looks like a value" "" "$phrase"
	done
	i=$((i + 1))
	fx_adr "$r" "$i" Accepted "Impact path with a size in it" '`cap-100MB.md`' "Plain."
	fx_seal "$r"
	printf '%s' "$r"
}

test_drift_count_field() {
	echo "drift count: impact field"
	local r
	r="$(fx_build_field)"
	assert_eq "a path on a continuation line is read" '"c/deep.txt changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 1) | .reason')"
	assert_eq "both members of a brace group with spaces are read" '"c/b.rs changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 2) | .reason')"
	assert_eq "a bullet list under an empty marker line is read" '"c/bullet.txt changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 3) | .reason')"
	assert_eq "the next field marker and a blank line end the field" "[1,2,3]" "$(drift_rows "$r" '.t3 | map(.number)')"
	assert_eq "a brace group leaves nothing unresolved" "[]" "$(drift_rows "$r" '.unresolved')"
	assert_eq "a backticked version is no path" "[6]" "$(drift_rows "$r" '.no_impact_adrs')"
}

test_drift_count_resolve() {
	echo "drift count: path resolution"
	local r
	r="$(fx_build_resolve)"
	assert_eq "a directory, a glob, a crate-relative path, a bare name and a deleted file are counted per file" \
		'["dir/p.txt","g/a.log","crate/src/commands/effective.rs","crate/src/commands/effective.rs","old/gone.txt"]' \
		"$(drift_rows "$r" '[.t3[] | .reason | split(" changed")[0]]')"
	assert_eq "they are ADRs 1 to 5" "[1,2,3,4,5]" "$(drift_rows "$r" '.t3 | map(.number)')"
	assert_eq "a directory spread over files and a partly resolved ADR are neither changed nor untouched" "[]" "$(drift_rows "$r" '.t4 | map(.number)')"
	assert_eq "a token that matches nothing or two files is listed with its ADR" \
		'[{"number":6,"token":"x/nothing.rs"},{"number":7,"token":"dup.rs"},{"number":8,"token":"zz/missing.txt"}]' "$(drift_rows "$r" '.unresolved')"
	assert_eq "an ADR whose tokens all fail is counted apart" 2 "$(drift_rows "$r" '.unresolved_adrs')"
}

test_drift_count_instants() {
	echo "drift count: commit instants"
	local r
	r="$(fx_build_instants)"
	assert_eq "a commit at the exact second of the ADR does not count" '"t/exact.txt changed in 3 commits after 2026-02-01T12:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 1) | .reason')"
	assert_eq "a commit with an older date inside the history does not hide an earlier one" '"t/skew.txt changed in 3 commits after 2026-02-01T12:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 2) | .reason')"
	assert_eq "an offset timestamp is the same instant as its Z form" '"t/off.txt changed in 3 commits after 2026-02-01T14:00:00+02:00"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 3) | .reason')"
}

test_drift_count_stamps() {
	echo "drift count: timestamps"
	local r
	r="$(fx_build_stamps)"
	assert_eq "a missing or unparseable timestamp excludes the ADR and is counted" '{"n":2,"adrs":[1,2]}' \
		"$(drift_rows "$r" '{n: .no_timestamp, adrs: .no_timestamp_adrs}')"
	assert_eq "an excluded ADR is no candidate of any type" "[3,4]" "$(drift_rows "$r" '[.t3[], .t4[], .t5[]] | map(.number) | unique')"
	assert_eq "a date-only timestamp is midnight Z of that day" '"d/day.txt changed in 3 commits after 2026-03-05"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 3) | .reason')"
	assert_eq "a record that is later as text but earlier as an instant does not clear" "[3,4]" "$(drift_rows "$r" '.t3 | map(.number)')"
	assert_eq "a record that is later as an instant clears" "[]" "$(drift_rows "$r" '.t3 | map(select(.number == 5))')"
	assert_eq "excluded ADRs stay in force" 5 "$(drift_rows "$r" .adrs_in_force)"
}

test_drift_count_naming() {
	echo "drift count: naming"
	local r
	r="$(fx_build_naming)"
	assert_eq "prose names a file by its exact path only; a directory or glob clears only from the impact field; an ambiguous suffix, an older and a same-instant record clear nothing" \
		"[1,3,4,8,9]" "$(drift_rows "$r" '.t3 | map(.number)')"
	assert_eq "a directory prefix without its slash and a truncated directory name name no file" '"src/a.rs changed in 3 commits after 2026-02-01T00:00:00Z"' \
		"$(drift_rows "$r" '.t3[] | select(.number == 1) | .reason')"
	r="$(fx_build_naming_kept)"
	assert_eq "a deleted file is named by its exact path in prose or in an impact field, a path with + or @ is named whole, and a backticked path with a line, line and column, anchor, item or call suffix names its file, while another suffix names nothing" \
		"[5,10]" "$(drift_rows "$r" '.t3 | map(.number)')"
}

glob_cleared() { # glob_cleared <name> <glob>: the files whose ADR a newer record with that impact glob clears, sorted
	local r i=0 file kept
	r="$(fx_build_glob_naming "$1" "$2")"
	kept="$(drift_rows "$r" '.t3 | map(.number)')"
	for file in "${FX_GLOB_FILES[@]}"; do
		i=$((i + 1))
		jq -e --argjson n "$i" 'index($n)' <<<"$kept" >/dev/null || printf '%s\n' "$file"
	done | sort
}

test_drift_count_globs() {
	echo "drift count: glob semantics"
	local r pattern n=0 repo
	repo="$(fx_build_glob_naming globref 'a?.md')"
	for pattern in 'cli/src/*.rs' 'cli/src/**/*.rs' 'a?.md'; do
		n=$((n + 1))
		assert_eq "naming by $pattern clears the files git lists for :(glob)$pattern" \
			"$(git -C "$repo" ls-files -- ":(glob)$pattern" | sort)" "$(glob_cleared "globn$n" "$pattern")"
	done
	r="$(fx_build_glob_adr globadr1 'cli/src/*.rs' cli/src/sub/n.rs)"
	assert_eq "a star does not cross a slash in an ADR impact glob" '{"t3":[],"t4":[1]}' "$(drift_rows "$r" '{t3: (.t3 | map(.number)), t4: (.t4 | map(.number))}')"
	r="$(fx_build_glob_adr globadr2 'cli/src/**/*.rs' cli/src/sub/deep/o.rs)"
	assert_eq "a double star crosses directories in an ADR impact glob" '"cli/src/sub/deep/o.rs changed in 3 commits after 2026-02-01T00:00:00Z"' "$(drift_rows "$r" '.t3[0].reason')"
	r="$(fx_build_glob_adr globadr3 'a?.md' abc.md a/b.md)"
	assert_eq "a question mark is one character within a segment" '{"t3":[],"t4":[1]}' "$(drift_rows "$r" '{t3: (.t3 | map(.number)), t4: (.t4 | map(.number))}')"
}

test_drift_count_patterns() {
	echo "drift count: value patterns"
	local r i expect
	r="$(fx_build_patterns)"
	i=0
	for expect in 'Node 20' 'Java 21' 'Python 3.11' 'PostgreSQL 15' 'v2.3.1' 'postgres:15' '$20 per month' '1000 per month' 'quota of 500' 'rate limit of 50 req/s' '100 MB' '3.9.0' '3.9.0'; do
		i=$((i + 1))
		assert_eq "ADR $i names exactly $expect" "$(jq -n --arg e "names $expect" '$e')" "$(drift_rows "$r" ".t5[] | select(.number == $i) | .reason")"
	done
	assert_eq "no look-alike number is flagged" "[1,2,3,4,5,6,7,8,9,10,11,12,13]" "$(drift_rows "$r" '.t5 | map(.number)')"
}

test_drift_count_sample() {
	echo "drift count: sample"
	local a c seed out outs
	a="$(fx_build_changes)"
	c="$(fx_build_values)"
	assert_eq "without --sample there is no sample" false "$(bash "$COUNT" "$a" | jq 'has("sample")')"
	out="$(bash "$COUNT" --sample 2 --seed 7 "$a" "$c")"
	assert_eq "a sample holds up to N candidates per type" '{"t3":2,"t4":1,"t5":2}' "$(jq -c '.sample | map_values(length)' <<<"$out")"
	assert_eq "each candidate has repo, number, title and reason" true \
		"$(jq '[.sample[][] | (keys == ["number","reason","repo","title"])] | all' <<<"$out")"
	assert_eq "the same seed gives the same bytes" "$out" "$(bash "$COUNT" --sample 2 --seed 7 "$a" "$c")"
	outs="$(for seed in 1 2 3 4 5 6 7 8 9 10; do bash "$COUNT" --sample 2 --seed "$seed" "$c" | jq -c '.sample.t5 | map(.number)'; done | sort -u | wc -l | tr -d ' ')"
	assert_true "another seed can choose other candidates" test "$outs" -gt 1
	assert_eq "a sample is a subset of the candidates" true "$(jq '[.sample.t5[].number] | all(. as $n | [1,2,3,4,5] | index($n))' <<<"$out")"
	assert_eq "the cap applies to the pool of all repositories" 3 "$(bash "$COUNT" --sample 3 --seed 1 "$c" "$c" | jq '.sample.t5 | length')"
	assert_eq "a cap above the pool returns every candidate" '[1,2,3,4,5]' "$(bash "$COUNT" --sample 100 --seed 1 "$c" | jq -c '[.sample.t5[].number] | sort')"
	assert_eq "a non-numeric size exits 2" 2 "$(
		bash "$COUNT" --sample many "$a" >/dev/null 2>&1
		echo $?
	)"
	assert_eq "the total reports every count, pooled over the repositories" '{"repos":2,"adrs_in_force":13,"no_timestamp":0,"unresolved_adrs":0}' \
		"$(jq -c '.total | {repos, adrs_in_force, no_timestamp, unresolved_adrs}' <<<"$out")"
}

test_drift_count_shape() {
	echo "drift count: shape"
	local a b c first second before after
	a="$(fx_build_changes)"
	b="$(fx_build_force)"
	c="$(fx_build_values)"
	first="$(bash "$COUNT" "$a" "$b" "$c")"
	second="$(bash "$COUNT" "$a" "$b" "$c")"
	assert_true "two runs print identical bytes" test "$first" == "$second"
	assert_eq "a result has the repo, the head and the docs directory" true \
		"$(jq --arg a "$a" '.repos[0] | .repo == $a and (.head | test("^[0-9a-f]{40}$")) and .docs_dir == "docs"' <<<"$first")"
	assert_eq "every array is sorted by number" true \
		"$(jq '[.repos[] | .t3, .t4, .t5 | (map(.number) == (map(.number) | sort))] | all' <<<"$first")"
	assert_eq "the total sums the repositories" '{"repos":3,"adrs_in_force":17,"no_impact_paths":'"$(jq '[.repos[].no_impact_paths] | add' <<<"$first")"',"no_timestamp":0,"unresolved_adrs":1,"t3":2,"t4":1,"t5":5}' \
		"$(jq -c '.total' <<<"$first")"
	assert_eq "the counts match the arrays" true \
		"$(jq '[.repos[] | (.counts.t3 == (.t3 | length)) and (.counts.t4 == (.t4 | length)) and (.counts.t5 == (.t5 | length))] | all' <<<"$first")"
	assert_eq "markdown prints one table per repository" 3 "$(bash "$COUNT" --markdown "$a" "$b" "$c" | grep -c '^## ')"
	assert_true "markdown prints a total line" grep -q '^total: ' <<<"$(bash "$COUNT" --markdown "$a" "$b" "$c")"
	before="$(git -C "$a" status --porcelain)$(git -C "$b" status --porcelain)"
	after="$(bash "$COUNT" "$a" "$b" >/dev/null; git -C "$a" status --porcelain)$(git -C "$b" status --porcelain)"
	assert_eq "the repositories are left unmodified" "$before" "$after"
	assert_eq "the repositories are clean" "" "$after"
}

test_drift_count_errors() {
	echo "drift count: errors"
	local r plain empty err
	r="$(fx_build_values)"
	assert_eq "an unknown option exits 2" 2 "$(
		bash "$COUNT" --bogus "$r" >/dev/null 2>&1
		echo $?
	)"
	plain="$TMP/drift-plain"
	mkdir -p "$plain"
	empty="$(fx_repo empty)"
	rmdir "$empty/docs/adr"
	assert_eq "no argument exits 2" 2 "$(
		bash "$COUNT" >/dev/null 2>&1
		echo $?
	)"
	err="$(bash "$COUNT" "$plain" 2>&1 >/dev/null)"
	assert_eq "a directory that is no git repository exits 2" 2 "$(
		bash "$COUNT" "$plain" >/dev/null 2>&1
		echo $?
	)"
	assert_true "and names the path" grep -qF "$plain" <<<"$err"
	err="$(bash "$COUNT" "$empty" 2>&1 >/dev/null)"
	assert_eq "a repository with no adr directory exits 2" 2 "$(
		bash "$COUNT" "$empty" >/dev/null 2>&1
		echo $?
	)"
	assert_true "and names the path" grep -qF "$empty" <<<"$err"
	mkdir -p "$r/handbook/adr"
	cp "$r/docs/adr/0001-fixture.md" "$r/handbook/adr/"
	assert_eq "--docs-dir reads another directory" '"handbook"' "$(bash "$COUNT" --docs-dir handbook "$r" | jq -c '.repos[0].docs_dir')"
	assert_eq "and counts only its records" 1 "$(bash "$COUNT" --docs-dir handbook "$r" | jq '.repos[0].adrs_in_force')"
}

test_drift_count() {
	test_drift_count_in_force
	test_drift_count_changed
	test_drift_count_untouched
	test_drift_count_values
	test_drift_count_field
	test_drift_count_resolve
	test_drift_count_instants
	test_drift_count_stamps
	test_drift_count_naming
	test_drift_count_globs
	test_drift_count_patterns
	test_drift_count_sample
	test_drift_count_shape
	test_drift_count_errors
}
