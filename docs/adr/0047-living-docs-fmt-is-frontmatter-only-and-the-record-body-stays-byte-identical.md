---
type: ADR
title: living-docs fmt is frontmatter-only and the record body stays byte-identical
description: fmt canonicalizes frontmatter only and leaves the record body byte-identical; the ADR 0046 reflow pass is removed, fmt accepts a record file path, and a --check dry-run reports pending changes.
owner: Evaldo Klock
status: Superseded
supersedes: 0046
superseded_by: 0071
timestamp: 2026-09-07T16:50:25Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0071](0071-fmt-repairs-a-record-s-cli-owned-fields-it-seeds-a-missing-timestamp-and-owner-and-aligns-the-title-heading-leaving-all-other-prose-byte-identical.md). Run `living-docs read` for what is in force.

# 0047. living-docs fmt is frontmatter-only and the record body stays byte-identical

This decision supersedes [ADR 0046](/adr/0046-fmt-unwraps-hard-wrapped-prose-one-paragraph-is-one-line-and-the-authoring-rule-says-so.md) and answers [issue 0041](/issues/0041-fmt-rewrites-record-bodies-reference-lists-collapse-to-one-line-and-supersede-emits-non-canonical-frontmatter-that-sends-users-to-fmt.md).

## Context

ADR 0046 added a body-reflow pass to `living-docs fmt`. The pass joined every hard-wrapped prose block into one line. Markdown renders consecutive non-blank lines as one paragraph, so the pass cannot tell a hard-wrapped paragraph from lines a reader means to keep separate. A `# References` list writes each entry on its own line with no blank line between entries, so the pass joined them into a single line.

`fmt` is also the verb `check` names to repair non-canonical frontmatter. A user who followed that advice ran a bundle-wide body rewrite. On the ai-configs bundle (314 docs) one bare `living-docs fmt` rewrote 179 records and deleted 12951 body lines. Recovery needed `git checkout -- docs`. A canonicalizer that loses body content on the documented happy path is unsafe by construction.

## Decision

We will make `living-docs fmt` frontmatter-only. `fmt` canonicalizes the frontmatter key order and leaves every byte below the closing `---` unchanged. We remove the ADR 0046 reflow pass and its module. `fmt` accepts a record file path as well as a bundle root, resolving the bundle from the file's ancestors. `fmt` gains a `--check` dry-run that reports which records would change and writes nothing.

## Consequences

**Easier / gained:**
- `fmt` is safe on any bundle; it never loses body content.
- A user can canonicalize one record without touching the rest of the bundle.
- `--check` makes a bundle-wide change visible before it is written.

**Harder / accepted trade-offs:**
- The "one paragraph is one line" authoring guideline loses its mechanical enforcer. Authors hold it by hand; `check` may advise but never rewrites a body.

**Follow-ups:**
- Wire the supersede link from ADR 0046 to this record with the fixed binary.
- Update the authoring-contract text that told users `fmt` unwraps prose.

## Verification

**Implementation impact:** `living-docs-core/src/commands/fmt.rs`, `living-docs-core/src/commands/fmt/reflow.rs` (removed), `living-docs-core/src/commands/fmt/tests.rs`, `cli/src/commands/fmt.rs`, `cli/tests/fmt.rs`.

**Verification criteria:**
- On an already-canonical bundle, `living-docs fmt` rewrites 0 records and leaves `git status` clean.
- A record whose body carries a five-line `# References` list keeps five lines after `fmt`.
- A body round-trip test parses a record, runs the fmt canonicalizer, and asserts the body bytes are unchanged.
