---
type: ADR
title: Doc-to-code drift is an advisory skill whose LLM judge runs outside the binary through a configurable command
description: Detect doc-to-code drift with a skill that resolves anchors and sends each in-scope record to a configurable LLM judge outside the binary, writing a dated advisory report.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-09T03:16:18Z
---

# 0075. Doc-to-code drift is an advisory skill whose LLM judge runs outside the binary through a configurable command

## Context

`living-docs check` validates doc-to-doc invariants only: numbering, liveness, moved sources, unfilled slots. Nothing tells a reader that a live record cites code that is gone, or describes behavior the code no longer has. A reader who follows such a record, agent or human, acts on a world that no longer exists.

Two earlier answers failed. [ADR 0032](/adr/0032-doc-code-lineage-is-declared-not-inferred.md) declared tree-sitter anchors and a failing `drift` verb; it is Deprecated. [ADR 0049](/adr/0049-mechanical-record-liveness-check-derives-stale-proposed-and-stale-impact-from-the-linked-issue-and-verification-paths.md) added a path-only `stale-impact` advisory, which [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md) removed: it had no true positive, and an Accepted ADR describes the code at the moment of its decision, so a path it cites that later disappears is history, not drift.

A read-only probe of this bundle (2026-11-08) confirmed it. Of the source paths cited in live records, 137 resolve and 19 do not; all 19 are in Accepted ADRs and are history (ADR 0059 cites the `docker-compose.yml` it removed), and none is in an open issue. Path resolution answers only half the question. The other half, whether the code still does what the record says, is the logic divergence that a deterministic check cannot see.

google/elephant-goldfish-model (commit 5382b17) solves the second half with an LLM skill that reads each design document with the code it cites and reports missing features, incomplete models, logic shortcuts and architectural divergence into a dated report. The agent-memory curator (its ADR 0155) shows a safe shape for that judgment: a versioned JSON payload in, JSON out, no tools, a command adapter whose argv comes from configuration, and the caller validating every answer.

This repository's own rule forbids that judgment inside the binary: [ADR 0001](/adr/0001-living-docs-cli.md) and the determinism boundary keep the CLI free of any LLM, and [ADR 0060](/adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md) fixes its surface at ten verbs.

## Decision

We will detect doc-to-code drift with a `living-docs-drift` skill that ships beside the `living-docs` skill and runs outside the binary. Its driver script reads the bundle, resolves the code anchors each in-scope record cites, sends one versioned JSON payload per record on stdin to a configurable judge command (default `claude -p --tools ""`; `codex exec`, pi or any script by configuration), validates every answer against a fixed schema, and writes a dated advisory report outside the bundle.

- **Scope.** Records kept in place are checked by their anchors and by the judge: architecture views, the constitution, and issues whose status is not terminal. Accepted ADRs get an adherence review by the judge only (does the code still follow this decision), never an anchor finding, because a path an ADR cites is history.
- **Anchors.** A backticked repository path, optionally followed by ` › ` and a backticked symbol. A path resolves when it exists inside the repository root; an absolute path, a `..` escape or a symlink leaving the root is reported unresolvable and never read. A symbol is normalized to its last `::` segment without `()` and reported as *name present* or *name absent* by a whole-word search of the cited file; the report never claims a symbol resolved, because a word search cannot tell a definition from a mention.
- **Judge contract.** The payload marks the record body and the code excerpts as data, never instructions. The judge's answer is a JSON object with a schema version, a verdict (`drift` or `no_drift`) and findings, each with a kind from EGM's four (missing feature, incomplete model, logic shortcut, architectural divergence), a summary and a cited path. An answer that fails the schema is reported as an invalid answer for that record, never as no drift.
- **Fail direction.** A missing judge command or `jq`, or a judge that fails, stops the run with exit 2 and no report, so an absent judgment is never read as a clean one. A written report exits 0 whatever it found: drift is advisory.
- **Report.** Written under `.living-docs/drift/<YYYY-MM-DD>-drift-report.md`, outside the bundle, so `check` never sees it. Its header names the judge command and the SHA-256 of each payload, which makes a run auditable although the judgment is not reproducible.
- **Never a gate.** No hook, pre-commit or CI job runs the skill.
- **The binary changes only to ship the skill.** `install skills` places the new skill's files like the other embedded skills; no verb, flag, JSON field or check pass is added. A `check --json` anchor field returns only with a true positive shown in a consumer bundle, the bar ADR 0057 set.

Rejected alternatives:

- **The judge command inside the binary** (a `drift` verb or a `check` flag running the configured command). It mirrors the curator most closely, but it puts non-reproducible judgment prose inside the tool that ADR 0001 keeps deterministic, and it adds a verb or flag to the surface ADR 0060 fixed.
- **A deterministic anchor advisory in `check` now.** It is the `stale-impact` class ADR 0057 removed for lack of a true positive, and the probe found none in the records it would cover.
- **A prompt-only skill, as EGM ships it.** No driver means the agent running it chooses the files, the format and whether a failed judgment counts as clean; the payload, the schema and the fail direction are the parts that make the result trustworthy.
- **A Rust front in the workspace** (ADR 0033). It would type the payload, but it adds a second release artifact for a tool that only shells out and reshapes JSON; a script beside the skill ships with the skill.

## Consequences

**Easier / gained:**
- A reader can ask whether live records still match the code and get a dated, auditable answer, including logic divergence no path check sees.
- The judge is swappable by configuration, and the binary stays deterministic and at ten verbs.

**Harder / accepted trade-offs:**
- Each run costs one judge call per in-scope record; the payload caps code excerpts to bound it.
- The skill needs `bash` and `jq` on the machine that runs it.
- The script reads record status from frontmatter and keeps the terminal issue statuses (`closed`, `done`) in its own list, a second copy of a registry fact; the registry stays the source of truth.

**Follow-ups:**
- Run the skill on a consumer bundle (se-configs) and, if it finds a true positive in a live record, open the issue for the `check --json` anchor field.

## Verification

**Implementation impact:** `skills/living-docs-drift/` (new: `SKILL.md`, the driver script, the judge prompt, tests), `cli/src/skill_install.rs` (one asset entry), the Makefile and CI test wiring.

**Verification criteria:**
- With a stub judge, the driver writes the report outside the bundle and `living-docs check` over the bundle stays green.
- A missing judge command, a failing judge and a judge answer that fails the schema each yield the outcome the Decision names, proved by the skill's test suite.
- An absolute, `..` or symlinked anchor path is reported unresolvable and its content never reaches the payload.
- `living-docs --help` lists the same verbs before and after the change: no verb is added.

# References

[1] [google/elephant-goldfish-model, egm-drift-report skill, commit 5382b17](https://github.com/google/elephant-goldfish-model)
