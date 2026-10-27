---
type: Issue
title: check and read grow quadratically with the bundle size
description: Remove the per-record whole-bundle work in check and read so they scale near-linearly, without a database.
status: open
timestamp: 2026-10-26T20:07:43Z
---

## 0097. check and read grow quadratically with the bundle size

`check` and `read` redo whole-bundle work once per record, so their time grows with the square of the record count. On a synthetic bundle of 5012 docs (2500 ADRs, 2500 issues, one supersede pair per ten ADRs), `check --json` takes 8.3 s and `read --json` 0.87 s; on 2512 docs they take 3.0 s and 0.31 s, and `read --json` takes 9.6 s on 20012 docs. This repository's own bundle (180 docs) runs `check` in about 0.3 s, so the cost appears only as a bundle grows.

A `sample` profile of `check` on 5012 docs (v0.24.0, macOS, release build) places the time in three spots:

- `check/graph.rs` › `is_listed_in` re-reads and re-parses the directory `index.md` for every record (about a third).
- `callout::successor_filename` lists and sorts the record's whole directory for every superseded record (about a third).
- Every check reads every record again through `store.read`, and each read canonicalizes the path and the bundle root (about a fifth).

`read` spends its time in `effective` › `supersede_lineage`, which rebuilds a map of every record for every record.

### Scope

- living-docs-core/src/check/graph.rs, living-docs-core/src/check/callout.rs, living-docs-core/src/callout.rs, living-docs-core/src/check/mod.rs.
- living-docs-core/src/commands/effective.rs.
- A bundle generator under scripts/bench, so the numbers can be reproduced.
- Kept out: a database, a search index, new verbs or flags, and `fmt`, which calls `successor_filename` the same way but has no target here.

### Acceptance

- `check --json` and `read --json` output is byte-identical to v0.24.0 on the 2512, 5012 and 20012-doc bundles and on this repository's `docs/`.
- `check` takes at most 1.5 s on 5012 docs, and its time on 5012 docs is at most 2.2 times its time on 2512 docs.
- `read --json` takes at most 300 ms on 5012 docs.
- Each number comes from an interleaved run against the retained v0.24.0 binary, with medians and spread, and the evidence goes in the pull request.

### Plan

Do less work, one measured slice at a time:

1. `check`: parse each directory index once, and resolve successors from one listing per directory instead of one per record.
2. `read`: build the record map once per run.
3. Measure. Only if `check` still misses 1.5 s, read each record once per run (a cache needs this number first).

### Outcome

All four slices delivered; every acceptance number is met and the output is byte-identical.

| Run, 5012 docs | v0.24.0 | This change | Gain |
|---|---|---|---|
| `check --json`, instructions | 151.9G | 25.7G | 5.9× |
| `check --json`, wall | 7.01 s (6.87–7.30) | 1.19 s (1.14–1.96) | 5.9× |
| `read --json`, instructions | 21.9G | 3.0G | 7.3× |
| `read --json`, wall | 0.86 s (0.82–1.31) | 0.18 s (0.17–0.19) | 4.8× |
| `check` peak RSS | 31.6 MB | 43.3 MB | +11.7 MB, the read-once map |
| `read` peak RSS | 79.9 MB | 31.4 MB | −48.5 MB |

- `check` on 2512 docs: 2.87 s → 0.61 s (0.57–0.64), 56.8G → 13.1G instructions. The 5012/2512 ratio is 1.95 in wall time and 1.97 in instructions, so growth is now linear.
- `read --topic` on 5012 docs: 0.65 s → 0.27 s; on this repository's `docs/`, 20 ms.
- Parity: `cmp` is silent for `check --json` and `read --json` (plus `--full`, `--contract` and `--topic`) on the 2512, 5012 and 20012-doc bundles and on `docs/`.

Slices:

1. `check`: each directory index is parsed once into its set of targets, and successors come from one listing per directory.
2. `read`: the record map is built once per run.
3. `read`: frontmatter scalars come from the YAML that is already parsed, and the topic fold is skipped when no terms are given. This touches `record.rs` and `commands/effective/topic.rs`, outside the planned scope.
4. `check`: S1 and S2 left `check` at 3.51 s, so a `ReadOnceStore` serves each record once per run. Its `write` and `rename` refuse, which makes "a check run never writes" enforced rather than documented. Sibling ids are a set. This touches `check/read_once.rs` and `check/records.rs`, outside the planned scope.

Evidence:

- Arms:
  - control: the v0.24.0 release build, sha256 prefix `be94f0270deb9b60`;
  - this change: the `task/ek-docs-db-ladder` working tree, release build, sha256 prefix `10c72f729eab6c0c`.
- Toolchain and machine: rustc 1.99.1, macOS 27.0.1, Apple M5 Max with 18 cores.
- Each A/B is 5 interleaved rounds with `scripts/bench/measure.sh`, counted by `/usr/bin/time -l`, over bundles from `scripts/bench/gen-bundle.py examples/linkly/docs <out> <N>` with N of 1250, 2500 and 10000.
- What bounds `check` now: the first read of each record from disk, measured with `sample` over 3 s of `check --json` on the 20012-doc bundle (2144 samples).
  - 1749 samples (82%) are under `ReadOnceStore::read` into `FsStore`.
  - Of those, 1291 are in the kernel `read`, 216 in `open` and 224 in the `realpath` behind `FsStore::confine`.
  - YAML parsing is under 5%.
  - The next hypothesis is fewer syscalls per record, for example confining against a root canonicalized once. It is not attempted here.

### Decision

- `ReadOnceStore` stands beside `LineEndingStore` as a second `DocStore` decorator. Their shared lines are the delegation the trait forces on every wrapper, so the duplication detector is settled in `.jscpd.json` rather than by renaming parameters to dodge it. Not taken: a shared delegation macro, which would hide the trait's shape from a reader for a few lines saved.
