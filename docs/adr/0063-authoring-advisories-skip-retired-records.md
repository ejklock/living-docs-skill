---
type: ADR
title: Authoring advisories skip retired records
description: The HEADING and SIZE advisories skip records that are Superseded or terminal for their type, through one retired-record predicate shared with set title
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-22T22:11:31Z
---

# 0063. Authoring advisories skip retired records

## Context

`check` prints advisories next to its violations. An advisory never fails the gate; it exists to name something the author can still improve. Two of them judge how a record is written: `HEADING` (the body heading disagrees with the frontmatter `title`, [ADR 0061](0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md)) and `SIZE` (the body exceeds its line target, issue 0009).

A retired record is history: its status is terminal for its type (`Deprecated` for an ADR, `closed` or `done` for an issue) or it is `Superseded`. ADR 0061 already draws that line for writes — `set title` refuses a retired record, because "a closed one is history". `check` does not draw it for reads. It keeps advising on records the tool refuses to change.

In this bundle, on 2026-09-22, `check` printed 54 advisories. 39 of them were anchored to retired records: 33 `HEADING` and 6 `SIZE`. None of the 39 can be acted on through the CLI, so they never go away. A reader who meets 54 findings and can act on 15 learns to skip the list, and then misses the 15.

## Decision

We will make the authoring advisories — `HEADING` and `SIZE` — skip every retired record, where retired means `status: Superseded` for any type or a status listed in that type's `terminal_statuses` in the doc-type registry. The predicate has one definition in `core`, shared by the two advisories and by `set title`'s refusal, so the record the CLI refuses to edit is exactly the record `check` stops judging.

Every other finding is unchanged. Invariant violations still run over retired records, because a broken link or a missing `superseded_by` in a retired record is a defect, not a style. `MOVED-SOURCE` still fires, because it warns a live record about its dependency, not about the retired one.

Rejected: keeping every advisory on every record. It is the current behavior; it costs nothing to keep, and it leaves 39 findings that the CLI forbids the author to fix.

Rejected: removing the `HEADING` advisory entirely. It is simpler, but it drops the drift signal ADR 0061 introduced on purpose, including on live records where `set title` does fix it.

## Consequences

**Easier / gained:**
- Every advisory `check` prints names a record the author can still change.
- The `HEADING` and `SIZE` advisories and `set title` share one definition of a retired record, so they cannot disagree about which records are history.

**Harder / accepted trade-offs:**
- A heading or size drift inside a retired record is no longer reported. That is accepted: the record is history and the CLI refuses to rewrite it.

**Follow-ups:**
- None. The live headings that disagree with their titles are aligned in [issue 0048](../issues/0048-check-stops-advising-on-retired-records-and-the-live-headings-align-with-their-titles.md), the issue that implements this decision.

## Verification

**Implementation impact:** `living-docs-core/src/check/records.rs` (`HEADING`), `living-docs-core/src/check/size.rs` (`SIZE`), the shared retired-record predicate in `core`, and `living-docs-core/src/commands/set/retitle.rs` (its refusal reuses the predicate).

**Verification criteria:**
- A `Deprecated`, `closed`, `done` or `Superseded` record whose heading disagrees with its title produces no `HEADING` advisory; the same record with an `Accepted` or `open` status produces one.
- A retired record over the size target produces no `SIZE` advisory; the same record while live produces one.
- A retired record with an invariant violation still fails `check`.
- `set title` on a retired record is still refused, through the shared predicate.

# References

[1] [ADR 0061 — set title retitles a record](0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md)
