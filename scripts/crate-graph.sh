#!/usr/bin/env bash
#
# crate-graph.sh — print the workspace crates and their in-workspace dependencies as a
# sorted Mermaid `graph TD`, for the generated region of docs/architecture/crates.md.
#
# Only normal and build dependencies count; a dev-dependency is not an edge. Nodes and
# edges are sorted, so an unchanged workspace yields the same bytes.
#
# Usage:  crate-graph.sh
# Exit:   0 = graph printed, non-zero = cargo metadata or jq failed.

set -euo pipefail

cargo metadata --format-version 1 --no-deps | jq -r '
	[.packages[].name] as $names
	| ["graph TD"]
	  + ($names | sort | map("  " + .))
	  + ([.packages[] | .name as $from | .dependencies[]
	      | select(.kind == null and (.name as $to | $names | index($to)))
	      | "  " + $from + " --> " + .name] | sort)
	| .[]'
