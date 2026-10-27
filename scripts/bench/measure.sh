#!/usr/bin/env bash
# usage: measure.sh <rounds> <bundle> <binary>... -- <args with BUNDLE placeholder>
# Interleaves the binaries round by round. The instruction and RSS counters
# come from macOS `time -l`, which the sandbox blocks, so run it outside one.
set -u
rounds=$1; bundle=$2; shift 2
bins=()
while [ "$1" != "--" ]; do bins+=("$1"); shift; done
shift
args=("${@//BUNDLE/$bundle}")
for r in $(seq 1 "$rounds"); do
  for bin in "${bins[@]}"; do
    out=$(/usr/bin/time -l "$bin" "${args[@]}" 2>&1 >/dev/null)
    real=$(printf '%s\n' "$out" | awk '/real/ {print $1}')
    ins=$(printf '%s\n' "$out" | awk '/instructions retired/ {print $1}')
    rss=$(printf '%s\n' "$out" | awk '/maximum resident set size/ {print $1}')
    printf '%s\t%s\treal=%s\tinstr=%s\trss=%s\n' "$r" "$(basename "$bin")" "$real" "$ins" "$rss"
  done
done
