---
type: ADR
title: Doc-code lineage is declared, not inferred
description: Every doc-code lineage fact is a declared event (trailer, covers glob, FQN anchor, explicit rename re-declaration) — the tool never infers links heuristically; staleness fails loud at the gate.
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-08-05T20:18:36Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0032. Doc-code lineage is declared, not inferred

## Context

[Research 0003](/research/0003-provenance-aware-knowledge-graph-for-living-docs.md) established three field facts. Plain-text code references in docs rot silently in most projects, and the decay driver is rename/move/deprecation refactoring. Automated (IR/ML) link recovery is not good enough to substitute for curated links, and human vetting degrades the best machine-generated link sets. In the field, links that were never created (47% of release artifacts) outnumber links that broke (12%) — creation friction is the larger enemy. Every shipped system that re-anchors positions after change (GitHub review comments, Gerrit ported comments, Swimm auto-sync) does it heuristically and accepts decay. [Issue 0024](/issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) already specifies the layered pairing design (commit trailers, `covers:` globs, executable acceptance). What it leaves open is the identity model: what a doc-code link points AT, and who resolves change.

## Decision

We will treat every doc-code lineage fact as a **declared event**, never an inference, and anchor links to **position-independent identities**:

- **Commit trailers (`Living-Doc: NNNN`, issue 0024 Layer 1) are the episode stream.** Each linked commit is a provenance episode in the ADR 0031 graph: a `implemented_by` edge from record to commit, `system_from` = sync time, `valid_from` = commit time.
- **Anchors point at identities, not positions.** Path-level anchors are `covers:` globs (issue 0024 Layer 2). Symbol-level anchors, when a record needs finer grain, are fully-qualified-name monikers in the SCIP style — no file line, no byte offset, resolved against a tree-sitter index at check time.
- **Renames are declared, not guessed.** When a covered path or anchored symbol is renamed, the link is re-bound by an explicit CLI event (`living-docs covers`/anchor re-declaration in the same commit), which closes the old edge's `valid_to` and opens a new edge. The tool never runs similarity heuristics to chase a moved symbol; that judgment belongs to the authoring model or the human, outside the determinism boundary.
- **Staleness is a loud, deterministic verdict.** `drift`/`verify` fail when an anchor no longer resolves or a covered path changed without a trailer. An unresolved anchor is an error to fix in the commit that caused it — never a score to decay silently.

Rejected alternative: heuristic auto-re-binding inside the tool (Swimm's model). It trades loud breakage for silent approximation — the exact failure mode the field data shows developers miss.

## Consequences

**Easier / gained:**
- Lineage queries ("why does this module exist", "which code implements ADR N") become graph traversals over declared edges with commit-grade provenance.
- Anchors survive every edit that does not rename the anchored identity; renames fail fast at the gate in the commit that caused them.
- Zero new friction beyond issue 0024: the trailer and glob mechanics are unchanged; this ADR fixes their identity semantics and graph representation.

**Harder / accepted trade-offs:**
- Rename ergonomics depend on the gate: a rename without re-declaration blocks at `drift`, and the author pays the re-binding cost at that moment.
- Symbol-level anchors need a tree-sitter identity source; until one ships, records anchor at path granularity only.
- No benchmark exists for ADR-to-code traceability; the design is validated by dogfooding telemetry, not literature.

**Follow-ups:**
- Issue 0024 Layers 1-2 implement the event stream and path anchors; a future issue specifies the symbol-anchor index.
- Record rename-frequency telemetry to measure real anchor half-life before tuning anything.

## Verification

**Implementation impact:** `cli` (`commits`/`verify`/`reconcile`/`drift` verbs), `living-docs-core` (anchor + trailer domain), `db-store/src/sync.rs` (edge materialization from trailers).

**Verification criteria:**
- A commit with a `Living-Doc: NNNN` trailer materializes exactly one `implemented_by` edge on next sync; re-sync is idempotent.
- Renaming a covered path without re-declaration makes `drift` exit non-zero; the same rename plus a re-declaration in the same commit passes and closes/opens edges bi-temporally.
- No code path in the tool computes similarity between an old and a new anchor target (fitness: grep-level assertion that no fuzzy-match dependency enters the workspace).

# References

[1] [Research 0003 — provenance-aware knowledge graph for living-docs](/research/0003-provenance-aware-knowledge-graph-for-living-docs.md)
[2] [Issue 0024 — doc-code pairing](/issues/0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md)
[3] [SCIP protobuf schema — position-independent symbol grammar](https://github.com/sourcegraph/scip/blob/main/scip.proto)
[4] [TAN, W.; WAGNER, S.; TREUDE, C. Detecting Outdated Code Element References in Software Repository Documentation](https://arxiv.org/abs/2212.01479)
