---
type: ADR
title: read --links-to and --impact answer who depends on a record from a link graph built in memory per run
description: read gains --links-to (direct inbound links) and --impact (inbound transitive closure with distance) over body links between live records, computed per run with no persisted index, returning the read view filtered
owner: Evaldo Klock
status: Accepted
timestamp: 2026-10-26T22:18:18Z
---

# 0074. read --links-to and --impact answer who depends on a record from a link graph built in memory per run

## Context

An agent about to change or supersede a record has to know which records depend on it. Today the only answer is `grep` over the bundle. ADR 0059 cut the database read-model, and it named `grep` and `read --topic` as the interim answer for search, not for dependency. A `grep` for a file name misses bundle-relative versus file-relative links, matches fenced examples, and cannot follow a chain: an issue that links an ADR that links the record under change is invisible.

`check` already extracts every record's links with a markdown parser (`check::links`, fence-aware, every link form) to judge their validity. Issue 0097 made `check` and `read` linear in the bundle size: on 5012 docs, `check` takes 1.19 s and `read --json` 0.18 s. A link graph built in memory per run therefore costs one parse per record, which `read` already pays. It needs no stored index.

The constitution keeps the binary free of a daemon and a database (ADR 0059). ADR 0060 fixes the ten verbs and makes each JSON shape a contract, where a field may be added but never renamed or removed.

## Decision

We will add two flags to `read`, answered from a link graph built in memory during the run and never persisted:

- **`read --links-to <TYPE/NNNN>`** returns the live records whose body links the named record. A link here is any markdown link or image destination that `check::links` extracts, resolved the same way `check` resolves it.
- **`read --impact <TYPE/NNNN>`** returns the inbound transitive closure: every live record that reaches the named record through a chain of links. Each record appears once, at its shortest distance, and carries a `distance` field (1 for a direct link). Cycles terminate.
- **The edges** are body links between records. `index.md` and `log.md` are not records, so they carry no edge. A record's link to itself is ignored. Frontmatter carries no edge; supersede lineage stays the job of the `read` view.
- **Live records only, in the result.** The result holds the same active set the plain `read` shows: no `Superseded` or `Deprecated` record. A retired record keeps its edges, so a chain passes through it: when D links a retired C and C links A, D is in A's impact at distance 2. The target itself may be a retired record, so "who still links the old one" has an answer. The target is never in its own result.
- **The output** is the `read` view, filtered. The record shape stays the same, and the flags compose with `--full` and `--contract`. `--links-to` keeps `read`'s ordering. `--impact` orders by distance, then by `read`'s ordering. Text output appends the distance to each line.
- **Usage.**
  - The two flags conflict with each other and with `--topic`.
  - A reference that names no record exits 2 with the reference in the message, as `set` does.
  - A record that nothing links returns an empty set and exits 0.

Rejected alternatives:

- **Leave dependency to `grep`.** Rejected: it cannot follow a chain, it confuses the two link forms, and it matches fenced examples. An agent ends up skipping the check instead of paying for it.
- **A new `graph` verb.** Rejected: it breaks the ten-verb surface of ADR 0060 to answer a question that is a filter over what `read` already compiles. The answer is a set of records, which is exactly what `read` returns.
- **A persisted index (`.living-docs/index.db`, FTS5 or an edge table).** Rejected: it amends the constitution's "no database" and supersedes ADR 0059 to save a per-run cost the measurement does not justify, and it adds a second state that can drift from the `.md` tree. The same reasoning keeps `read --topic` lexical.
- **An edge-list output (`{from, to}` pairs).** Rejected: it adds a second JSON contract to keep and does not compose with `--full` or `--contract`. The caller wants the records, not the graph.
- **Counting supersede lineage as edges (a link to an ancestor counts as a link to its head).** Rejected: it mixes two relations into one answer, and `check`'s moved-source advisory already reports a link to a superseded record.

## Consequences

**Easier / gained:**
- Before a supersede or an edit, one deterministic command lists what to revisit, with how far away each dependent is.
- There is no new state to keep fresh: the graph is a pure function of the `.md` tree at the moment of the run.

**Harder / accepted trade-offs:**
- Every `--links-to` or `--impact` run re-extracts the links of every record, which is linear in the bundle. That is acceptable at the measured sizes; a bundle where it is not would reopen the persisted-index question with a number.
- `distance` becomes part of the JSON contract under `--impact`.
- In a densely linked corpus, `--impact` can return most of the bundle. `distance` is how a caller trims it.

**Follow-ups:**
- [Issue 0098](/issues/0098-read-links-to-and-impact-list-the-records-that-depend-on-a-record.md) carries the implementation.

## Verification

**Implementation impact:**
- A link-graph module in `living-docs-core`, reusing the `check::links` extraction and resolution.
- `commands/effective.rs` for the filter and the ordering.
- `cli/src/args/sub.rs` (`ReadArgs`) and the `read` help.

**Verification criteria:**
- On a fixture where B and C link A, and D links B, `--links-to` A returns {B, C} and `--impact` A returns B and C at distance 1 and D at 2.
- A cycle (A links B, B links A) terminates. A retired linker, a fenced link, an index link and a self-link add nothing. An unknown reference exits 2.
- `read --links-to` and `read --impact` on the 5012-doc bench bundle take at most 300 ms, measured with `scripts/bench/measure.sh`.
- Fitness function: the plain `read --json` output stays byte-identical to its output before the change.
