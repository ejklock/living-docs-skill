---
type: Issue
title: "Corpus import: seed the knowledge graph read-model from real repos to make graph slices demoable from day one"
description: Sync real repos into one read-model (multi-project sync exists; verify it) and add a deterministic `import` verb for non-conformant bundles, so every knowledge-graph slice demos against a real corpus instead of fixtures.
status: closed
timestamp: 2026-08-05T20:25:27Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Corpus import: seed the knowledge graph read-model from real repos to make graph slices demoable from day one

The knowledge-graph work ([ADR 0031](../adr/0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md), [ADR 0032](../adr/0032-doc-code-lineage-is-declared-not-inferred.md)) needs real data before its first query verb lands: an edge model validated only against fixtures proves conformance, not usefulness. Multi-project sync already exists (`db sync --docs-dir <path> --project <slug>`, ADR 0005), so a conformant bundle can enter a shared read-model today. What is missing is (a) a verified multi-repo corpus workflow, (b) a deterministic import path for repos whose docs are NOT living-docs-conformant, and (c) making that corpus the standing demo target for every graph slice. This also feeds the calibration-pending items of [issue 0024](0024-doc-code-pairing-for-living-docs-commit-trailers-covers-based-drift-detection-and-executable-acceptance.md) with real usage data.

Part of [PRD 0001](../prd/0001-living-docs-atlas-multi-project-authoring-wiki-over-living-docs-core.md) (multi-project read-model); feeds ADR 0031/0032.

### Scope

- **Conformant repos (verify, fix, document):** `db sync --project <slug> --engine sqlite` for N repos into one `.living-docs/index.db`; cross-project `search` returns hits with project attribution; `relations` rows stay project-scoped. Whatever breaks under real multi-repo usage gets fixed here.
- **Non-conformant repos (new, deterministic):** `living-docs import <src-dir> --type <token> --project <slug>` maps foreign docs into records by mechanical rules only: filename `NNNN-slug` gives number and slug; first `#` heading gives title (filename stem as fallback); existing frontmatter keys map when they match; everything unmappable lands in the EAV tail; imported records carry a CLI-owned `imported: true` marker. The USER names the doc type per directory — the tool never classifies a document (determinism boundary, ADR 0001). The import ends with a report: N mapped, M skipped, each skip with its reason.
- **Corpus as demo harness:** a documented smoke flow (script or make target) that syncs the chosen repos and runs the current demo query set; every future graph slice (edge widening, `why`/`impact`/`as-of`, trailers) demos against this corpus, not against fixtures.
- KEPT: the authoring contract and `check` strictness for native bundles — import relaxes MAPPING, never invariants; a synced conformant bundle passes the same checks as today.
- Out of scope: LLM-assisted classification or content rewriting of imported docs (authoring-model work, ADR 0001); write-back to the source repos (import is read-only toward the source); code-symbol nodes (later slice per ADR 0032).

### Acceptance

- Two or more real repos sync into one SQLite read-model; `living-docs search` returns cross-project hits attributed to the right project slug.
- Importing a non-conformant docs directory produces records with correct number/title/type from mechanical rules alone, an `imported: true` marker, and a mapping report listing every skipped file with a reason; re-import is idempotent.
- Supersede relations present in imported conformant bundles appear as `relations` rows scoped to their project.
- The demo smoke flow runs end-to-end from a clean checkout: sync all corpus repos, execute the demo queries, exit zero.
- A native bundle synced through this path passes `check` unchanged (no invariant relaxation leaks).

### Plan

1. Corpus smoke: pick 2-3 real repos (this repo + at least one downstream consumer), verify multi-project sync + cross-project search, fix what breaks, document the flow. Two defects already observed in the first single-repo smoke (2026-08-05): a hyphenated query (`bi-temporal`) aborts `search --engine sqlite` with an FTS5 syntax error (`no such column: temporal`) — user queries need sanitizing or quoting before they reach MATCH; and the default project slug derives from the docs dir name (every repo becomes project `docs`) — the default should derive from the repo directory name, or multi-repo corpora will collide on the first sync without `--project`.
2. `import` verb for one non-conformant repo with the mechanical mapping + report; idempotence test.
3. Wire the corpus into the graph slices' demo ACs (each ADR 0031/0032 slice demos against the corpus from this point on).
