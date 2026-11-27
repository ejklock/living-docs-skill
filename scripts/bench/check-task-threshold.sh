#!/usr/bin/env bash
# usage: check-task-threshold.sh [binary] [cutoff seconds]
# Doubles the synthetic bundle size until the median of three `check` runs
# reaches the cutoff (default 2 s, Claude Code's default task poll interval),
# then bisects between the last size under it and the first at or above it
# until the step is under 5 % of the in-force record count. Each line prints
# the in-force record count (the count the MCP server decides on, as
# `read records` lists it) beside the median. Bundles live under $TMPDIR and
# are removed on exit.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
bin=${1:-$root/target/release/living-docs}
cutoff=${2:-2}
work=$(mktemp -d "${TMPDIR:-/tmp}/check-task-threshold.XXXXXX")
trap 'rm -rf "$work"' EXIT
bundle=$work/docs

# Builds the bundle for $1 records per type; sets $records and $median.
measure() {
  python3 "$here/gen-bundle.py" "$root/examples/linkly/docs" "$bundle" "$1"
  "$bin" --docs-dir "$bundle" manage index >/dev/null
  "$bin" --docs-dir "$bundle" manage fmt >/dev/null
  records=$("$bin" --docs-dir "$bundle" --json read records |
    python3 -c 'import json,sys; print(len(json.load(sys.stdin)["records"]))')
  median=$(for _ in 1 2 3; do
    start=$(date +%s.%N)
    "$bin" --docs-dir "$bundle" check >/dev/null
    echo "$(date +%s.%N) - $start" | bc -l
  done | sort -n | sed -n 2p)
  printf '%s\t%s\t%s\n' "$1" "$records" "$median"
}

reaches() { [ "$(echo "$median >= $cutoff" | bc -l)" = 1 ]; }

printf 'host\t%s\n' "$(grep -m1 'model name' /proc/cpuinfo | cut -d: -f2- | sed 's/^ //')"
printf 'binary\t%s\n' "$("$bin" --version)"
printf 'date\t%s\n' "$(date +%F)"
printf 'per_type\tin_force_records\tmedian_seconds\n'
low=0; low_records=0; high=50
while :; do
  measure "$high"
  if reaches; then break; fi
  low=$high; low_records=$records; high=$((high * 2))
  [ "$high" -le 25600 ] || { echo "no size up to 25600 per type reached ${cutoff}s" >&2; exit 1; }
done
high_records=$records
while [ $(( (high_records - low_records) * 20 )) -ge "$high_records" ]; do
  mid=$(( (low + high) / 2 ))
  measure "$mid"
  if reaches; then high=$mid; high_records=$records; else low=$mid; low_records=$records; fi
done
printf 'threshold\t%s\n' "$high_records"
