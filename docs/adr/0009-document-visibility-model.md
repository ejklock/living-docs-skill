---
type: ADR
title: Document visibility is default-deny frontmatter data, validated and index-aware
description: Adopt a visibility frontmatter field (private/public/showcase, absent means private) that check validates and the index can filter, so which docs are publishable is human-declared data an LLM never re-judges.
status: Accepted
supersedes:
superseded_by:
tags: [documentation, visibility, publishing, methodology]
timestamp: 2026-07-17T21:29:53Z
---

# 0009. Document visibility is default-deny frontmatter data, validated and index-aware

## Context

Living Docs mixes docs that are safe to publish (the method: constitution, most ADRs,
templates) with docs that are the moat and must never leave (research, accumulated lessons,
client specifics, in-flight decisions). Today the corpus has no machine-readable signal for
which is which, so any "publish the public part" step would have to *judge* each doc at publish
time — LLM-as-judge on a privacy boundary, the exact failure mode a publishing pipeline must
forbid.

The `ai-configs` fork of living-docs already solved this: a `visibility` frontmatter field,
decided once by the human at authoring time, recorded as data, and enforced deterministically
forever. This repo (the Rust-CLI fork) lacks it entirely — `check` does not know the field and
`index` cannot filter by it.

The forces: (1) the decision "is this doc public?" must be made by a human, once, and never
re-litigated by an LLM in the publish path; (2) omission must be safe — forgetting the field can
never publish something by accident; (3) the value must be an *instrument*, not a vibe — a typo
like `pubic` must fail a check, not silently pass; (4) it is the prerequisite for a
visibility-aware index and a future `public-export`.

## Decision

We will adopt a **`visibility: private | public | showcase`** frontmatter field on OKF concept
docs, **default-deny**:

- **Absent ⇒ private.** Omission is the safe default; a doc with no `visibility` is never
  publishable. Templates therefore do **not** carry a `visibility:` line — omission is the
  intended private default, and seeding it in templates would invite an accidental non-private
  value.
- **`check` validates the domain.** If the field is present it must be exactly one of
  `private` / `public` / `showcase`; any other value (a typo, an invented level) is a check
  violation. Absent is valid (no violation). This is the fitness function that keeps the field an
  instrument.
- **The value is mechanical; the choice is authoring judgment.** Deciding *which* docs are public
  is the human's, made when the doc is written and confirmed. `check` only enforces the domain,
  never *which* value a doc should carry.
- **The index can filter by it.** `index --visibility <csv>` lists only docs whose visibility is
  in the set (absent ⇒ private), so a public bundle's index never links a private doc — by
  construction, no dangling link and no leak. Omitting the flag lists every doc (the dev view,
  unchanged from today).

This is the data foundation. The `public-export` capability that consumes it (allowlist build +
leak gate + clean-history publish) is a separate decision, recorded in its own ADR when built.

## Consequences

**Easier / gained:**
- A machine-readable, human-declared answer to "is this doc publishable" that no LLM re-judges at
  publish — the privacy decision is data, set once.
- A visibility-aware index becomes possible: a public listing that cannot link a private doc.
- Unblocks `public-export` (a future ADR) without that skill having to judge privacy itself.

**Harder / accepted trade-offs:**
- Authors must set `visibility` on docs intended to be public; the default-deny means a doc stays
  private until someone deliberately elevates it (friction sits on the dangerous direction, by
  design).
- A new optional field the `check` and `index` code must understand (small, additive).
- Default-deny can surprise ("why isn't my doc in the public index?") — mitigated by documenting
  the field in the conventions and making the index flag explicit.

**Follow-ups:**
- ADR 0010 (when built): `public-export` — allowlist build, deterministic leak gate, human-gated
  clean-history publish — consuming this field.
- The visibility validation and the index filter each ship as their own vertical slice with tests.

## Verification

**Implementation impact:** `living-docs-core/src/check/records.rs` (validate the `visibility`
domain), `living-docs-core/src/commands/index.rs` + `cli/src/main.rs` (the `--visibility` filter),
and the skill conventions (`SKILL.md`, a rules file, `rules/doc-language.md` controlled-value list,
templates guidance).

**Verification criteria:**
- `check` exits non-zero on a doc carrying `visibility: pubic` (or any value outside the domain)
  and passes on `private` / `public` / `showcase` and on an absent field. — fitness function
  (test in `check/records.rs`).
- `living-docs index adr --visibility public,showcase` renders only public/showcase ADRs; without
  the flag every ADR is listed. — fitness function (integration test).
- `living-docs check docs` stays green over `docs/` with this ADR indexed.
