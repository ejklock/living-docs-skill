---
type: Issue
title: read --links-to and --impact list the records that depend on a record
description: "Implements ADR 0074: direct and transitive inbound links over live records, filtered read view, built per run"
status: closed
timestamp: 2026-10-26T22:18:43Z
---

## 0098. read --links-to and --impact list the records that depend on a record

Implements [ADR 0074](/adr/0074-read-links-to-and-impact-answer-who-depends-on-a-record-from-a-link-graph-built-in-memory-per-run.md). `read --links-to <TYPE/NNNN>` lists the live records that link a record, and `read --impact <TYPE/NNNN>` lists every live record that reaches it through a chain of links, each with its distance. This answers "what do I revisit before I change or supersede this" without `grep`.

### Scope

- A link-graph module in `living-docs-core`, reusing the extraction and resolution in `check::links`. It is built per run and never persisted.
- The filter and ordering in `commands/effective.rs`, the two flags in `ReadArgs`, and the `read` help and examples (ADR 0060).
- The living-docs skill text that tells an agent to check dependents before a supersede.
- Kept out: a stored index, a new verb, frontmatter or lineage edges, and composing with `--topic`.

### Decision

- The reference is `TYPE/NNNN`, parsed the way `set` parses it. A shorter form was not taken because one form per CLI is the rule.
- The flags conflict with `--topic` instead of intersecting with it. Relaxing a conflict later is additive; narrowing a composition is breaking.
- The FTS5 index of the ladder (step 2) is not built. Lexical `read --topic` takes 20 ms on this repository and 0.27 s on 5012 docs (issue 0097). The embeddings step had no recall evaluation to gate on, so it is closed with it.
- Under `--impact`, text output ends each index line with `[distance N]` and puts `_distance N_` under the heading of a `--full` block. Not taken: a distance column, which would change the plain index line's layout.

### Acceptance

- On a fixture where B and C link A, and D links B:
  - `read --links-to adr/A` returns exactly {B, C} in `read` order;
  - `read --impact adr/A` returns B and C with `distance` 1, then D with `distance` 2.
- These add nothing to either result:
  - a cycle, which terminates;
  - a `Superseded` or `Deprecated` linker;
  - a link inside a fenced block;
  - a link from `index.md`;
  - a record's link to itself.
- A retired target still returns its live linkers.
- An unknown reference exits 2 and names it.
- `--links-to` with `--impact`, or either one with `--topic`, exits 2.
- `--full` and `--contract` compose with both flags.
- Text output shows the distance under `--impact`.
- Plain `read --json` (no new flag) is byte-identical to its output before the change, on the 5012-doc bench bundle and on `docs/`.
- On the 5012-doc bench bundle, each flag takes at most 300 ms, measured with `scripts/bench/measure.sh` over 5 interleaved rounds against the control.

### Plan

1. Core: the link graph (inbound edges between live records) plus the direct and transitive queries, with unit tests for every case in Acceptance.
2. Front: the flags, usage errors, text and JSON rendering, help, and the skill line, with binary tests.
3. Measure on the bench bundle and record the numbers in Outcome.

### Outcome

Delivered as specified in ADR 0074. The flags are `read --links-to TYPE/NNNN` (direct inbound links) and `read --impact TYPE/NNNN` (the inbound closure, nearest first, each record with its `distance`).

- The graph is built in memory per run, from body links between records. A retired record contributes its edges, so a chain passes through it, but it never appears in the result.
- Both flags compose with `--full` and `--contract`. Text output shows `[distance N]` on index lines and `_distance N_` under the heading in `--full`.
- An unknown reference exits 2 and names it. The two flags conflict with each other and with `--topic`.
- Tests: 17 core cases in `living-docs-core/src/commands/effective/tests/dependents.rs` and 8 binary tests in `cli/tests/read_dependents.rs`.

Measured on the 5012-doc bench bundle (`scripts/bench/measure.sh`, release build, 5 interleaved rounds):

| Run | Wall | Instructions |
|---|---|---|
| `read --json --links-to adr/0001` | 0.18–0.21 s | 3.55–4.13 G |
| `read --json --impact adr/0001` | 0.19–0.22 s | 3.56–3.75 G |
| plain `read --json` | 0.16 s | 2.93–3.00 G |

Both flags stay within the 300 ms budget. The extra instructions over a plain `read` come from extracting every record's links. The limiter is the same per-record read and parse that bounds plain `read`, not the graph walk.

Plain `read --json` and `read --json --full` are byte-identical to the build before this change, on `docs/` and on the 2500- and 10000-doc bundles.
