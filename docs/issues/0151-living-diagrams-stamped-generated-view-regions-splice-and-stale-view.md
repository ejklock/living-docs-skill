---
type: Issue
title: "Living diagrams: stamped generated view regions, splice and stale-view"
description: "Implements ADR 0101: manage view --splice stamps a generated region with its inputs hash, check advises stale-view, and a runner script regenerates; this repository dogfoods a crate-graph view."
status: open
timestamp: 2026-11-23T02:38:10Z
---

## 0151. Living diagrams: stamped generated view regions, splice and stale-view

Implements [ADR 0101](../adr/0101-generated-view-regions-carry-a-hash-of-their-inputs-stamped-by-manage-view-splice-and-checked-for-staleness.md), continuing the generated-diagram work of [issue 0144](0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

### Scope

- `manage view <view> --splice --inputs <glob>...`: Mermaid from standard input into the view's generated region, stamped with its inputs and their hash.
- A `stale-view` advisory in `check` for a stamped region whose inputs changed.
- A runner script in the skill that runs each marker's command, splices the output, and with `--check` fails on a diff.
- This repository's own `component` view of its crate graph, generated through the runner and checked in `make check`, as the first measured use.
- The skill rule rewritten for the stamped marker, with per-stack generator recipes backed by research 0005.

Kept: hand-drawn view kinds, the existing unstamped marker (still valid, never checked for staleness), and the rule that the binary never runs an extractor.

### Decision




- Living diagrams (owner, 2026-11-22, after the advisor's design review): the runner script plus an in-binary splice and a `stale-view` advisory (ADR 0101). Not taken: keeping the recipe alone; the runner script alone with the binary unchanged (the session's recommendation); an in-binary graph snapshot, which needs a parser per language.
- Stamp only narrow inputs: the recipe tells authors to stamp a schema, migrations, compose, OpenAPI or manifests, and to leave import-graph views to the runner's `--check` in CI, since an inputs hash flags them on nearly every source commit. Not taken: refusing a stamp on an import-graph kind in the binary, which would make the binary judge a view's kind.
- Measure over 20 pull requests on this repository: diff failures that were real drift versus noise, and seconds added to `make check`.
- Input walk (2026-11-23): the walk starts at the repository root, prunes every folder no glob can match below, and never descends into `.git`; an unreadable folder no glob can reach is never visited, while one a glob could reach fails the hash loudly. Not taken: walking the whole repository per call (155 ms on this repository with a built `target/`, and one unreadable folder anywhere failed every hash).
- Symlinks in the inputs (owner, 2026-11-23): a symlink is hashed as git stores it, its path and its target text, never followed; a checkout without symlink support writes that text as a plain file and gets the same hash. Not taken: following links that stay inside the repository, which read `.git` through a committed link; skipping symlinks, which misses a re-pointed link.
- Ignored files (owner, 2026-11-23): the walk skips what `.gitignore` excludes, through the `ignore` crate, as git-aware build tools hash only tracked-or-trackable files. Not taken: narrow globs only, which still pull generated files into a `**` glob.

### Acceptance

- `manage view <view> --splice --inputs 'Cargo.toml' '*/Cargo.toml' < graph.mmd` writes the region and the stamp; a second identical run writes no byte; the view still passes `check` and its Mermaid parses.
- Changing a stamped input makes `check` report `stale-view` naming the view and its globs, exit 0; changing only the view's prose does not.
- A glob that is absolute, holds `..`, or resolves through a symlink out of the repository root is refused by `--splice` and skipped by `check`.
- The runner regenerates every generated region of the bundle and, with `--check`, exits non-zero when one would change.
- This repository holds a `component` view of its crates, kept current by `make check`.

### Plan

- S1: the marker grammar parser and the inputs hash (pure, in core), with the guarded file reads through the store port | files: living-docs-core (new module), fs-store/src | after: none
- S2: `manage view --splice` in the CLI and the MCP `manage` tool | files: cli/src/args, cli/src/commands (new module), mcp/src | after: S1
- S3: the `stale-view` advisory | files: living-docs-core/src/check (new module), check/kinds.rs, skills/living-docs/rules/check.md | after: S1 and issue 0150 S4 (both touch kinds.rs)
- S4: the runner script, this repository's crate-graph view, `make check` wiring, and the rewritten skill rule with recipes | files: skills/living-docs/scripts (create), skills/living-docs/rules/architecture-diagrams.md, docs/architecture (create), Makefile | after: S2
