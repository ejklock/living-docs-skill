---
type: Issue
title: Make the CLI deterministic across releases and close the paths that leave a bundle red or silently wrong
description: Implements ADR 0094, 0095 and 0096 and fixes the supersede, hook and write-path defects the 2026-11-20 audit reproduced, with a property test and model-driven evals.
status: open
timestamp: 2026-11-20T16:31:10Z
---

## 0149. Make the CLI deterministic across releases and close the paths that leave a bundle red or silently wrong

Implements [ADR 0094](../adr/0094-generated-retired-record-text-names-no-command-so-renaming-a-verb-never-invalidates-a-bundle.md), [ADR 0095](../adr/0095-a-new-check-rule-starts-as-an-advisory-and-becomes-a-violation-only-by-a-later-adr-that-counts-it-over-real-bundles.md) and [ADR 0096](../adr/0096-check-advises-on-an-inline-code-repository-path-that-no-longer-exists.md). An audit on 2026-11-20 ran every write verb in scratch bundles and surveyed 43 real bundles: 26 failed `check`, and the CLI itself could leave a bundle wrong with `check` green.

### Scope

Close every path found by which the CLI, or its hook, leaves a bundle red, or wrong and green:

- Generated callout and index-note text that changes with the release (ADR 0094).
- `write set <record> status <value>` accepted on a record that carries `superseded_by`: the link stays, the callout goes, `check` is green and `read records` lists both records.
- `write supersede` across two types: exits 0, writes a bare number that resolves to the wrong record and leaves `check` red.
- `write supersede` writes two records with no rollback, and every record write is a plain overwrite.
- The pre-commit hook in changed-only mode passes a stale type index when the staged file is a record of that type.
- Four hand-written states pass `check` in silence (ADR 0095).
- No rule tells an author that a code path a record names is gone (ADR 0096).
- No test asserts that an arbitrary sequence of write verbs leaves `check` green.
- No measure of how a model or a person fares with the CLI.

Kept: the verb surface, the registry, every existing violation, and the rule that no model runs inside the binary.

### Decision


- Callout text (owner, 2026-11-20): verb-free text, one canonical form. Not taken: verb-free text plus accepting the earlier wordings; keeping the wording and accepting the earlier ones.
- New rules (owner, 2026-11-20): born advisories, measured over the real bundles before any promotion. Not taken: violations at once; only `stale-successor` as a violation.
- Code anchor (owner, 2026-11-20): a `path-missing` advisory in `check`. Not taken: the drift skill only; a `covers:` key.
- Commits (owner, 2026-11-20): branch `task/ek-0149-deterministic-cli`, one commit per approved delivery, no push.
- Status on a superseded record: `write set status` refuses any status change on a record that carries `superseded_by`, as it already refuses a title change on a terminal record. Not taken: clearing the link and the callout, which would be an un-supersede verb nobody asked for.
- Cross-type supersede: refused. Not taken: writing a typed reference into `superseded_by`, a frontmatter contract change.
- Changed-only scope (owner, 2026-11-20): a stale type index is in scope when any record of that type is among the changed files, which inverts what `changed_files_reports_drift_only_when_the_drifted_index_is_among_them` asserted; the pre-commit hook also passes staged deletions. Not taken: keeping the earlier contract and fixing only staged deletions; reporting only drift the commit itself caused, which makes check depend on git; failing on every unstaged drift (the session's first recommendation).
- Read-only target: the atomic write refuses an existing file that is not writable, as the in-place write did. Not taken: replacing it through the rename, which silently overwrites a file its owner protected.
- No un-supersede path: a supersede is undone by hand; the eval scenario for that request passes when the bundle is left uncorrupted. Open for the owner: whether a verb should undo a supersede.
- Evals: a runner command set by an environment variable drives the model, as the drift skill does for its judge; no model call lives in the workspace crates.

### Acceptance

- A fixture bundle holding each earlier callout and index-note wording is green after `manage fmt` and `manage index`, and a second run changes no byte.
- `write set <record> status Accepted` on a superseded record exits non-zero and changes no file.
- `write supersede adr/N issues/M` exits non-zero and changes no file.
- A `write supersede` whose second record write fails leaves both records as they were.
- With changed-only on, a commit that stages a record whose type index is stale fails.
- Each of the five new advisory kinds fires on its state and leaves the report `ok`.
- A property test over generated sequences of write verbs ends every sequence with `check` green.
- The eval scenarios run against a stub runner in `make check`, and one recorded run with a model reports, per scenario, the final `check` verdict, the CLI calls that failed and the hand edits to CLI-owned fields.
- Counts of each new advisory kind over the 43 surveyed bundles are recorded in this issue's Outcome.

### Plan

- S1: the retired-record callouts and the index history note name no command, and `manage fmt` plus `manage index` migrate each earlier wording | files: living-docs-core/src/callout.rs, living-docs-core/src/commands/index/rows.rs, their tests and fixtures, skills/living-docs/rules/semantic-index.md, skills/living-docs/templates/claude-hard-rules.md, skills/living-docs-drift/tests/fixtures | after: none
- S2: `write set status` refuses a record that carries `superseded_by` | files: living-docs-core/src/commands/set.rs, a new sibling test file | after: none
- S3: `write supersede` refuses two records of different types and rolls back when a later write fails | files: living-docs-core/src/commands/supersede | after: none
- S4: a record write replaces the file through a temporary file and a rename | files: fs-store/src | after: none
- S5: the advisory kinds status-vocabulary, record-type, timestamp and stale-successor | files: living-docs-core/src/check (kinds.rs, mod.rs, new modules), new cli tests | after: none
- S6: changed-only scope takes in the type index of a staged record | files: living-docs-core/src/check/report.rs or cli/src/commands/check.rs, new cli test | after: none
- S7: eval harness: scenarios, a runner set by environment variable, a scorer over the session transcript, stub runner in `make check` | files: evals (create), Makefile | after: none
- S8: the path-missing advisory | files: living-docs-core/src/check, fs-store/src, the store port | after: S4, S5
- S9: property test over sequences of write verbs | files: cli/tests (create), cli/Cargo.toml, Cargo.lock | after: S1, S2, S3
- S10: migrate this bundle with the new binary, document the five kinds in the skill rules, measure the kinds over the surveyed bundles, run the evals with a model and a terminal walkthrough | files: docs, skills/living-docs/rules | after: S1, S5, S7, S8
