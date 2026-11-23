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
- No un-supersede path: a supersede is undone by hand; the eval scenario for that request passes when the bundle is left uncorrupted.
- Evals: a runner command set by an environment variable drives the model, as the drift skill does for its judge; no model call lives in the workspace crates.
- Atomic write (owner, 2026-11-20): dropped; record writes stay a plain overwrite. Review showed that replacing by rename trades a partial read for a directory listing that can miss the record for an instant under a concurrent writer, changes the owner of another user's file and breaks a hard link, and the defect that motivated it is covered by the supersede rollback. Not taken: keeping it with the trade-offs documented; keeping it with a retried listing. This supersedes the "Read-only target" bullet above and removes slice S4.
- Sequence test: bounded exhaustive enumeration with std only. Not taken: `proptest`, a dev-dependency tree and a random seed in a project whose point is reproducibility.
- Path probe: the existence probe of ADR 0096 is a default method on the store port that answers "unknown" unless the adapter implements it, so every existing store double keeps compiling and the rule is skipped where no repository root exists. Not taken: a second port passed to `check::compile`, which changes every caller's signature.
- Type token case (2026-11-22, after the owner asked to close the minor items before release): a CLI type token matches its registry row ignoring ASCII case (`ADR/0001`, `write new ADR`), in `doc_type::spec_for` only; a directory on disk still matches exactly. Not taken: keeping the refusal with a "did you mean" hint, which still costs the caller a retry.
- Cross-type refusal wording: the message names both types without an article, so no type reads "a ADR". Not taken: choosing the article per type, which a new registry row would have to remember.
- Undoing a supersede (owner, 2026-11-22): stays by hand, guided by the status refusal. Not taken: a `write unsupersede` verb, a public surface with no measured use.
- Subproject-relative paths in path-missing (owner, 2026-11-22): a path still resolves from the repository root only, and those findings stay advisories. Not taken: accepting a path found under any first-level subdirectory, which hides a path gone from the root; a configured list of search roots, a new config contract.
- Advisory promotion (owner, 2026-11-22): `stale-successor` becomes a violation by its own ADR, since it has 0 hits over the surveyed bundles; the other kinds stay advisories. Not taken: no promotion before release; promoting `status-vocabulary` too, with 319 hits.
- Stale successor on a terminal record (2026-11-22, review of the promotion slice): a Deprecated record or a closed issue that carries `superseded_by` also fails `check`, as ADR 0098 states for any status other than Superseded, but its message says the status is not Superseded without claiming the record reads as in force, since `read records` already withholds it. Not taken: staying silent on terminal statuses, which leaves a link no verb wrote and `write supersede` would never write.

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
- S11: a type token in any case resolves to its registry row | files: living-docs-core/src/doc_type.rs, living-docs-core/src/doc_type/tests.rs, cli/tests (new file) | after: none
- S12: the cross-type supersede refusal names both types without an article | files: living-docs-core/src/commands/supersede/same_type.rs, the test that asserts its text | after: none
- S13: the sequence search fails when its call into the contract skips a part | files: fs-store/tests/write_sequences.rs, fs-store/tests/write_sequences/ | after: none
- S14: the constitution names the shipped verbs (`read records`, four verbs and `mcp`) | files: docs/constitution.md | after: none
- S15: `stale-successor` is a check violation (ADR 0098) | files: living-docs-core/src/check, cli/tests/check_frontmatter_advice.rs, fs-store/tests/write_sequences, skills/living-docs/rules/check.md | after: S13

### Outcome

- Retired-record callouts and the index history note name no command, and `manage fmt` plus `manage index` migrate every earlier wording idempotently (ADR 0094, S1 `67e2551`).
- `write set status` refuses a record that carries `superseded_by`; the refusal names both hand edits that undo a supersede and ends with `living-docs manage fmt` and `living-docs manage index` (S2 `5f22ffe`, S10 `a83ba65`). No verb undoes a supersede; the owner kept that by hand.
- `write supersede` refuses two records of different types and restores both records when a later write fails (S3 `054edba`). The atomic write (S4) was dropped by the owner; record writes stay a plain overwrite.
- Advisories status-vocabulary, record-type, timestamp and stale-successor (S5 `42d6b0c`) and path-missing (S8 `98b44fa`, ADR 0096) are born advisories (ADR 0095). Changed-only scope takes in the type index of a staged or deleted record (S6 `040519f`, ADR 0097).
- Counts over the surveyed bundles (42 measurable copies of `docs/`), new binary after `manage fmt` and `manage index`: status-vocabulary 319, timestamp 187 before `fmt` (0 after), record-type 7, stale-successor 0. Green bundles: 16 with v0.30.0 (1040 violations), 11 with the new binary before migrating (1088), 19 after migrating (553); what stays red is missing authorship, not CLI drift. path-missing over five repositories: 414 findings before refusing absolute and `..` tokens, 260 after, every sampled finding a path absent from disk; about 8% are paths relative to a subproject.
- An exhaustive test runs every sequence of up to three of 17 write operations from three starting bundles (empty, two ADRs and an issue, a superseded pair), 5219 sequences each, and holds the contract; removing the status guard turns it red (S9 `874bd3c`). The whole file takes about 40 s on a quiet host; at a load average above 30 it took up to 165 s, over its 120 s budget, with or without the last change.
- A terminal walkthrough (`.living-docs/qa/deterministic.walkthrough.json`, git-ignored, report in the owner's reports folder) replays the upgrade migration, the status refusal with its undo instructions, the cross-type refusal and the path-missing advisory.
- Evals (S7 `e7446e0`) with an isolated runner holding only the installed hooks: Sonnet 5.5 passed 8 of 8 scenarios, Haiku 5.5 7 of 8. Across the 16 transcripts: 0 unknown subcommands, 0 bare-number references, a read before every first write, 1 hand edit of a CLI-owned field (Haiku, reviving a superseded ADR; `check` caught it, and S10 rewrote the refusal for it).
- Before release (S11 `c8bff4d`, S12 `976dfdb`, S13 `785a6ce`, S14 `30b2efa`): a type token resolves in any ASCII case (`ADR/0001`, `write new Issue`) while a directory on disk and Unicode look-alikes still match exactly; the cross-type refusal names both types without an article; the sequence search fails when its call into the contract skips a part; the constitution names the shipped verbs.
- `stale-successor` is a violation (ADR 0098, S15 `e28c9e3`): a record with `superseded_by` and a status other than Superseded fails `check` with an invariant-4 message, and a terminal record is told its status is not Superseded without being called in force. No committed bundle in this repository holds the state.
- Owner decisions on the open points (2026-11-22): undoing a supersede stays by hand; path-missing keeps resolving from the repository root; only `stale-successor` was promoted.
