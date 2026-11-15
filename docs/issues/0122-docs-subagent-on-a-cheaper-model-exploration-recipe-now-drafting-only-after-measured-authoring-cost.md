---
type: Issue
title: "Docs subagent on a cheaper model: exploration recipe now, drafting only after measured authoring cost"
description: Owner decisions on delegating doc exploration and drafting to a cheaper-model subagent, gated by ADR 0001's measured-cost rule
status: open
timestamp: 2026-11-15T15:51:39Z
---

## 0122. Docs subagent on a cheaper model: exploration recipe now, drafting only after measured authoring cost

### Scope

The owner asked whether living-docs should define a docs subagent on a cheaper model (Haiku 5.5 by default, overridable in the user's configs) that explores and writes records under the session agent's review. The cheaper-render split is already listed as deferred and evidence-gated: [ADR 0001](/adr/0001-living-docs-cli.md) says it must rest on measured authoring cost, and `docs/constitution.md` lists it out of scope. The evidence that rationale lives in the session's context and cannot be recovered afterwards is in [research 0005](/research/0005-engineering-documentation-in-ai-assisted-development-living-architecture-domain-language-and-rules-decision-capture-issue-waves-and-a-local-html-view.md). This issue is a cost experiment, kept apart from the capability roadmap in [issue 0121](/issues/0121-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

Plan:
- S1: baseline: measure the share of session output tokens spent on record bodies over a sample of real authoring sessions | files: none in the binary | after: none
- S2: a harness-neutral skill topic with the exploration recipe: the subagent reads the bundle and code and returns `TYPE/NNNN` refs with quotes; the session reads every cited record before deciding | files: skills/living-docs corpus | after: none

### Decision

- Who drafts a record body: the session, as today. Not taken: a cheap subagent drafting from the session's brief (the brief already carries most of an ADR, and cheaper writing invites the inflation ADR 0057 removed); a cheap subagent for low-rationale sections only (revisit after the S1 baseline).
- Where delegation lives: the consumer's own harness configs plus a skill topic. Not taken: `manage install agents` (harness-specific placement that reverses issue 0046 and bakes a vendor model ID into an embedded asset); a configurable command on the ADR 0075 pattern (kept as the shape if drafting is ever proven).
- Default model: none in living-docs; the harness and the user's configs decide. Not taken: Haiku 5.5 hard-coded; an env var with a default inside a command script.
- Order: measure first, as ADR 0001 requires. Not taken: build first.
- Mechanical edits stay with the CLI, and review stays with the session; a cheaper model never reviews the session.

### Acceptance

- S1 reports the share of session output tokens spent on record bodies; if it is small, drafting delegation stops there and this issue closes on the exploration recipe alone.
- If drafting is reopened, it goes through an ADR and a constitution amendment before code, measured by cost-weighted tokens per accepted record, review rounds per draft, records edited within 14 days, invented claims per draft and records per merged issue.
- The exploration recipe is judged by governing records it missed or got wrong.
