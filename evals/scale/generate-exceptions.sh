#!/usr/bin/env bash
#
# generate-exceptions.sh — write the exception scenarios into a scenarios directory for run.sh.
#
# usage: generate-exceptions.sh OUT [SEED]        (SEED defaults to 153)
#
# An exception is a record about one topic that reverses one clause of a record about another
# topic, in prose, with no supersede. Every scenario holds "Upload handling" at ADR 3, six clauses,
# one of them "limit every upload to 25 MB". x1 adds, late in the numbering, "Mobile carrier
# limits", which lowers that limit to 8 MB; control xcontext adds the same record naming ADR 3 as
# context only; control xnone adds nothing. The task asks for the upload limit in
# config/limits.env: 8 follows the exception, 25 follows the clause it left dead. Unprompted only,
# at two bundle sizes. The same SEED and sizes always write the same files.
#
# environment: LIVING_DOCS_SCALE_SIZES  two bundle sizes in ADRs (default: "60 240")
# Exit: 0 = written, 2 = bad usage.

set -euo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() {
	printf 'generate-exceptions: %s\n' "$*" >&2
	exit 2
}

readonly UPLOADS=$'Upload handling\tEvery upload is checked, limited, scanned, kept and named the same way.\tUsers upload files through the web app and the mobile app.\taccept uploads in PNG, JPEG and PDF; limit every upload to 25 MB; scan every upload for malware; keep every upload for 90 days; name every stored file by its content hash; reject any upload without an owner'

readonly UPLOADS_REF="ADR $(printf '%04d' 3)"

exception_fields() { # exception_fields <exception|context>: the late record, tab-joined, naming the upload record
	case "$1" in
	exception) printf '%s\t%s\t%s\t%s' "Mobile carrier limits" "Requests stay under the size carrier proxies let through." \
		"Carrier proxies drop requests over 8 MB, and most uploads now come from phones." \
		"lower the upload size limit of $UPLOADS_REF to 8 MB for every upload, because carrier proxies drop larger requests" ;;
	context) printf '%s\t%s\t%s\t%s' "Mobile carrier limits" "Mobile uploads resume in chunks when a carrier proxy drops them." \
		"Carrier proxies drop long requests, and most uploads now come from phones." \
		"resume mobile uploads in 2 MB chunks after a dropped request; the upload rules of $UPLOADS_REF stay as they are" ;;
	esac
}

readonly TASK="Configure the service: write the upload size limit the decisions in docs/ set into config/limits.env as one line UPLOAD_MAX_MB=<number>. Do not change the docs."

write_scenario() { # write_scenario <name> <type> <size> <wording>
	local dir="$OUT/$1-unprompted" late=$(($3 - 2)) plant="" expected=25
	mkdir -p "$dir"
	[[ "$4" == exception ]] && expected=8
	[[ "$4" == xcontext ]] && plant="$(exception_fields context)"
	[[ "$4" == exception ]] && plant="$(exception_fields exception)"
	awk -v seed="$SEED" -v size="$3" -v plant_pos="$([[ -n "$plant" ]] && echo "$late" || echo 0)" -v plant="$plant" \
		-f "$HERE/manifest.awk" "$HERE/concerns.tsv" "$HERE/components.txt" |
		awk -F'\t' -v OFS='\t' -v uploads="$UPLOADS" '
			$1 == "adr" && $2 == 3 { print "adr", 3, uploads; next }
			($1 == "supersede" && ($2 == 3 || $3 == 3)) || ($1 == "deprecate" && $2 == 3) { next }
			{ print }' >"$dir/manifest.tsv"
	printf 'TYPE=%s\nSIZE=%s\nWORDING=%s\nPOSITION=late\nARM=unprompted\nPLANT=3\nEXPECTED_LIMIT=%s\n' "$2" "$3" "$4" "$expected" >"$dir/scenario.env"
	printf '%s\n' "$TASK" >"$dir/task.md"
	printf '%s\n' '#!/usr/bin/env bash' 'set -euo pipefail' '. "$(dirname "$0")/../lib.sh"' 'scale_setup "$(cd "$(dirname "$0")" && pwd)"' >"$dir/setup.sh"
	printf '%s\n' '#!/usr/bin/env bash' 'set -uo pipefail' '. "$(dirname "$0")/../lib.sh"' 'scale_expect_exception "$(cd "$(dirname "$0")" && pwd)"' >"$dir/expect.sh"
}

main() {
	(($# >= 1 && $# <= 2)) || die "usage: generate-exceptions.sh OUT [SEED]"
	OUT="$1"
	SEED="${2:-153}"
	[[ "$SEED" =~ ^[0-9]+$ ]] || die "SEED must be a number: $SEED"
	read -r SMALL LARGE EXTRA <<<"${LIVING_DOCS_SCALE_SIZES:-60 240}"
	[[ "$SMALL" =~ ^[0-9]+$ && "$LARGE" =~ ^[0-9]+$ && -z "${EXTRA:-}" && SMALL -ge 6 ]] || die "LIVING_DOCS_SCALE_SIZES needs two sizes of at least 6"
	mkdir -p "$OUT"
	cp "$HERE/../scenarios/common.sh" "$HERE/lib.sh" "$OUT/"
	local size
	for size in "$SMALL" "$LARGE"; do
		write_scenario "x1-$size-exception-late" x1 "$size" exception
		write_scenario "control-$size-xcontext-late" control "$size" xcontext
		write_scenario "control-$size-xnone-late" control "$size" xnone
	done
}

main "$@"
