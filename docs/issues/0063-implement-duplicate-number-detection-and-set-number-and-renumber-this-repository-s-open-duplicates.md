---
type: Issue
title: Implement duplicate-number detection and set number, and renumber this repository's open duplicates
description: "Implements ADR 0069: check fails on two records of one type sharing a number, set <ref> number <NNNN|next> renames file, heading, links and index, and the seven open duplicate issues here move to free numbers"
status: open
timestamp: 2026-10-24T10:17:26Z
---

## 0063. Implement duplicate-number detection and set number, and renumber this repository's open duplicates

Implements [ADR 0069](/adr/0069-duplicate-record-numbers-fail-check-and-set-number-renumbers-a-record-with-its-links-and-index.md).

### Scope

- `check` reports a violation when two records of the same numbered type share a number, naming both paths and the repair `living-docs set <path> number next`.
- `set <ref> number <NNNN|next>` renames the file with its slug kept, rewrites the heading number and every in-bundle link to the old path, and regenerates the type index; any status is accepted.
- `set --help`, the procedure and issue-workflow topics and the README name the key and the repair.
- This repository's seven open issues that share a number with a closed one (0055 to 0061) are renumbered with `set <path> number next`, using the binary built from this change.

### Decision

- The rename and link rewrite reuse the retitle machinery; option not taken: a second rename path.
- The open record of each duplicate pair moves and the closed one keeps its number, since commits and releases cite the closed ones.

### Acceptance

- Two indexed records with one number in a type make `check` exit 1 with one violation naming both paths; distinct numbers pass.
- `set <path> number next` takes the next free number, renames the file, rewrites the heading and every in-bundle link, and `check` passes with no `index` call; `index` afterwards writes nothing.
- A taken number, a malformed number, and an architecture view or the constitution exit 2 and change nothing.
- `ls docs/issues | grep -oE '^[0-9]{4}' | sort | uniq -d` prints nothing, and `living-docs check docs` passes.
- The full suite, `make check` and `se-gates check` pass.

### Plan

1. Core: the duplicate-number check and its kind; tests red first.
2. Core and CLI: the `number` key over the retitle machinery; tests through the binary.
3. Help and topics; renumber the seven open duplicates.
