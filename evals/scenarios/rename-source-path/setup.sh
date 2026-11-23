#!/usr/bin/env bash
#
# setup.sh — a decision that names a source file by its old path in inline code.

set -euo pipefail
. "$(dirname "$0")/../common.sh"

eval_start
mkdir -p src/store
printf 'pub fn get() {}\n' >src/store/cache.rs
eval_adr "Cache reads in process" "Reads are cached in the process." 'cache reads in the process, in `src/cache.rs`, with a short expiry' >/dev/null
eval_seal
