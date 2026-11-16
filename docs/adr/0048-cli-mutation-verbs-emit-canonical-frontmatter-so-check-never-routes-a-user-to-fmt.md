---
type: ADR
title: CLI mutation verbs emit canonical frontmatter so check never routes a user to fmt
description: supersede, status, and new write frontmatter through the canonical serializer to_canonical_markdown, so their output always passes check's canonical invariant and never routes a user to fmt.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-07T16:50:28Z
---

# 0048. CLI mutation verbs emit canonical frontmatter so check never routes a user to fmt

This decision answers the second defect in [issue 0041](../issues/0041-fmt-rewrites-record-bodies-reference-lists-collapse-to-one-line-and-supersede-emits-non-canonical-frontmatter-that-sends-users-to-fmt.md) and pairs with [ADR 0047](0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md).

## Context

`check` treats a record as canonical only when its frontmatter matches `record.rs::to_canonical_markdown` byte-for-byte. `supersede`, `status`, and `new` do not use that serializer. They edit frontmatter with targeted line insertions. `supersede` inserts an absent key (`superseded_by`, `supersedes`) at the close of the frontmatter block, after `timestamp`, but the canonical order places those keys before `tags` and `timestamp`. So `supersede` produces frontmatter that `check` reports as `non-canonical (hand-written?) frontmatter -- run living-docs fmt`.

That message sent users to `fmt`, the verb that (before ADR 0047) rewrote bodies. A CLI verb must never emit output that its own gate rejects.

## Decision

We will route `supersede`, `status`, and `new` through the canonical serializer `to_canonical_markdown`. Each verb mutates the record's parsed frontmatter fields, then re-emits the whole frontmatter block canonically. The body below the closing `---` stays byte-identical. After any of these verbs, `check` reports no non-canonical-frontmatter finding.

## Consequences

**Easier / gained:**
- A CLI verb's output always passes `check`; the tool never advises `fmt` after its own write.
- One serializer owns key order. A new frontmatter key is ordered in one place.

**Harder / accepted trade-offs:**
- These verbs now re-emit the full frontmatter block, not a single line. The body-preservation invariant must be tested, because re-emission touches more bytes than a line edit.

**Follow-ups:**
- None; ADR 0047 covers the fmt side of issue 0041.

## Verification

**Implementation impact:** `living-docs-core/src/commands/supersede/mod.rs`, `living-docs-core/src/commands/status/mod.rs`, `living-docs-core/src/commands/new.rs`, `living-docs-core/src/record.rs`.

**Verification criteria:**
- After `living-docs supersede 0134 0146`, `living-docs check` reports no non-canonical-frontmatter finding for either record without running `fmt`.
- A supersede / status / new integration test asserts the mutated record's body bytes are unchanged and `check` passes.
