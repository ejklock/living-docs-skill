#!/usr/bin/env bash
#
# generate.sh — write the scale scenarios into a scenarios directory for run.sh.
#
# usage: generate.sh OUT [SEED]        (SEED defaults to 153)
#
# OUT gets common.sh and lib.sh, then one directory per scenario and arm: T1 (the task records a
# decision opposite to an in-force ADR), T2 (the task reverses one of two clauses of an in-force
# ADR) and a control with no plant, at two bundle sizes, the planted ADR worded with the task's
# words (same) or paraphrased (para), early or late in the numbering. Two noise controls (control
# with wording noise, one per size) hold a non-conflicting ADR that shares rare words with a task
# that conflicts with nothing: any supersede or link to a record in force fails them. Each scenario exists as
# NAME-unprompted (the task alone) and NAME-prompted (the task plus a request for CONFLICTS.txt).
# The same SEED and sizes always write the same files.
#
# environment: LIVING_DOCS_SCALE_SIZES  two bundle sizes in ADRs (default: "60 240")
# Exit: 0 = written, 2 = bad usage.

set -euo pipefail
export LC_ALL=C

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() {
	printf 'generate: %s\n' "$*" >&2
	exit 2
}

plant_fields() { # plant_fields <t1|t2> <same|para>: title, description, context and decision, tab-joined
	case "$1-$2" in
	t1-same) printf '%s\t%s\t%s\t%s' "Keep user sessions in Redis" "Every user session lives in a shared Redis cluster that all app nodes read." \
		"App nodes come and go behind the load balancer, so a user session must survive a node." \
		"keep every user session in a shared Redis cluster that all app nodes read and write" ;;
	t1-para) printf '%s\t%s\t%s\t%s' "Shared key-value store for login state" "Login state for every visitor sits in one central key-value cluster." \
		"Web servers come and go behind the router, so login state must outlive any one server." \
		"hold the login state of every visitor in one central key-value cluster that all web servers share" ;;
	t2-same) printf '%s\t%s\t%s\t%s' "Store uploads in S3 and serve them through the CDN" "Uploaded files are stored in S3 and served to users through the CDN." \
		"Users upload files that other users download, and downloads come from every region." \
		"store every uploaded file in the S3 bucket ld-uploads-153 and serve uploaded files to users through the CDN" ;;
	t2-para) printf '%s\t%s\t%s\t%s' "Object storage and edge delivery for user files" "Files people send us live in the object store bucket and reach readers from the edge cache network." \
		"People send us files that others then fetch, and those fetches come from every region." \
		"keep every file people send us in the object store bucket ld-uploads-153 and deliver those files to readers from the edge cache network" ;;
	control-noise) printf '%s\t%s\t%s\t%s' "Reconcile the settlement ledger every night" "The settlement ledger is reconciled against the bank statements every night." \
		"Finance needs the settlement ledger to match the bank statements before the morning." \
		"reconcile the settlement ledger against the bank statements every night" ;;
	esac
}

task_text() { # task_text <t1|t2|control> <wording>
	[[ "$1-$2" == control-noise ]] && {
		echo "We decided to email the settlement ledger reconciliation report to the finance mailbox at 06:00. Please record this decision in the docs."
		return
	}
	case "$1" in
	t1) echo "We decided that each app node keeps user sessions in its own memory, with sticky routing at the load balancer so a user always reaches the same node. Please record this decision in the docs." ;;
	t2) echo "We decided to serve uploaded files to users directly from S3 through short-lived signed URLs. Please record this decision in the docs." ;;
	control) echo "We decided to rotate the on-call engineer every week, with the handover on Monday at 10:00. Please record this decision in the docs." ;;
	esac
}

readonly PROMPTED_ASK="Also check whether this decision conflicts with any record already in the docs: write the number of each conflicting record into a file CONFLICTS.txt at the repository root, one per line, and leave the file empty when nothing conflicts."

write_manifest() { # write_manifest <dir> <size> <plant position> <type> <wording>
	local plant=""
	if [[ "$4" != control ]]; then plant="$(plant_fields "$4" "$5")"; elif [[ "$5" == noise ]]; then plant="$(plant_fields control noise)"; fi
	awk -v seed="$SEED" -v size="$2" -v plant_pos="$3" -v plant="$plant" \
		-f "$HERE/manifest.awk" "$HERE/concerns.tsv" "$HERE/components.txt" >"$1/manifest.tsv"
}

write_scenario() { # write_scenario <name> <type> <size> <wording> <position> <arm>
	local dir="$OUT/$1-$6" pos=0 plant
	mkdir -p "$dir"
	case "$5" in early) pos=3 ;; late) pos=$(($3 - 2)) ;; esac
	write_manifest "$dir" "$3" "$pos" "$2" "$4"
	plant="$pos"
	[[ "$2" == control ]] && plant=0
	printf 'TYPE=%s\nSIZE=%s\nWORDING=%s\nPOSITION=%s\nARM=%s\nPLANT=%s\n' "$2" "$3" "$4" "$5" "$6" "$plant" >"$dir/scenario.env"
	{
		task_text "$2" "$4"
		[[ "$6" == prompted ]] && printf '\n%s\n' "$PROMPTED_ASK"
	} >"$dir/task.md"
	printf '%s\n' '#!/usr/bin/env bash' 'set -euo pipefail' '. "$(dirname "$0")/../lib.sh"' 'scale_setup "$(cd "$(dirname "$0")" && pwd)"' >"$dir/setup.sh"
	printf '%s\n' '#!/usr/bin/env bash' 'set -uo pipefail' '. "$(dirname "$0")/../lib.sh"' 'scale_expect "$(cd "$(dirname "$0")" && pwd)"' >"$dir/expect.sh"
}

write_both_arms() { # write_both_arms <type> <size> <wording> <position>
	local name="$1-$2"
	[[ "$1" == control && "$3" != noise ]] || name="$name-$3-$4"
	write_scenario "$name" "$1" "$2" "$3" "$4" unprompted
	write_scenario "$name" "$1" "$2" "$3" "$4" prompted
}

main() {
	(($# >= 1 && $# <= 2)) || die "usage: generate.sh OUT [SEED]"
	OUT="$1"
	SEED="${2:-153}"
	[[ "$SEED" =~ ^[0-9]+$ ]] || die "SEED must be a number: $SEED"
	read -r SMALL LARGE EXTRA <<<"${LIVING_DOCS_SCALE_SIZES:-60 240}"
	[[ "$SMALL" =~ ^[0-9]+$ && "$LARGE" =~ ^[0-9]+$ && -z "${EXTRA:-}" && SMALL -ge 6 ]] || die "LIVING_DOCS_SCALE_SIZES needs two sizes of at least 6"
	mkdir -p "$OUT"
	cp "$HERE/../scenarios/common.sh" "$HERE/lib.sh" "$OUT/"
	write_both_arms t1 "$SMALL" same early
	write_both_arms t1 "$SMALL" para late
	write_both_arms t1 "$LARGE" same late
	write_both_arms t1 "$LARGE" para early
	write_both_arms t2 "$SMALL" same late
	write_both_arms t2 "$SMALL" para early
	write_both_arms t2 "$LARGE" same early
	write_both_arms t2 "$LARGE" para late
	write_both_arms control "$SMALL" none none
	write_both_arms control "$LARGE" none none
	write_both_arms control "$SMALL" noise early
	write_both_arms control "$LARGE" noise late
}

main "$@"
