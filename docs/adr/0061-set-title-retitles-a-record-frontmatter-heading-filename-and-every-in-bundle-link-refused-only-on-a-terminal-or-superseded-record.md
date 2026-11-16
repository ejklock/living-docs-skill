---
type: ADR
title: "set title retitles a record: frontmatter, heading, filename and every in-bundle link, refused only on a terminal or superseded record"
description: set gains a title key that rewrites frontmatter, heading, filename and every in-bundle link in one transaction, refused only on a terminal or superseded record
owner: Evaldo Klock
status: Accepted
timestamp: 2026-09-22T02:37:50Z
---

# 0061. set title retitles a record: frontmatter, heading, filename and every in-bundle link, refused only on a terminal or superseded record

## Context

A title is the one piece of frontmatter with no CLI path to correct it. `living-docs set` covers `status`, `description` and `owner`, and the authoring rule forbids hand-editing frontmatter, so a record born with a wrong title has only two exits: `supersede`, which spends a number and a retired-record callout on a typo, or delete-and-recreate through `new`, which also rewrites the timestamp. [Issue 0047](../issues/0047-set-accepts-title-so-a-live-record-can-be-retitled-without-hand-editing-frontmatter.md) reports the case that forced this: a `Proposed` ADR whose title carried a count the decision had already changed.

Three forces bound the answer. First, the filename slug is derived from the title, so a retitle that leaves the file alone creates a divergence the gate would have to tolerate forever. Second, records reference each other by path, not by number: 255 local markdown links in this bundle resolve to `<type>/NNNN-<slug>.md`, and `check::links` fails on every one that a rename would orphan. The `supersedes`/`superseded_by` frontmatter is immune — it carries the bare number — and `index` regenerates its own rows, so the link set is the whole exposure inside the bundle. Third, a reference that lives outside the bundle — a README in another repo, a code comment, a published GitHub URL — is beyond any scan the tool can run, and that exposure grows the moment a record is accepted and cited.

## Decision

We will make `title` a settable key on `set`, and define the retitle as one transaction over four artifacts: the frontmatter `title`, the record's numbered heading line, the file's name, and every in-bundle markdown link whose destination resolved to the old path. The verb refuses a title whose slug collides with an existing record, and refuses to act on a record whose status is terminal for its type or `Superseded` — an accepted decision may be retitled, a closed one is history. What lies outside the bundle is named, never edited: the verb reports how many in-bundle references it rewrote and hands the author the exact search for the rest. `check` gains the matching finding: a heading that disagrees with its record's `title` is an advisory, not a violation, because 45 of this bundle's 119 records carry a deliberately shortened heading and a corpus that predates the rule must not fail the gate.

The port grows with the verb: `DocStore` gains a required `rename`, chosen over a `remove` plus a write at the new path because the filesystem gives us one atomic operation and a two-step rewrite can leave two copies of a record in the tree.

Rejected: gating the retitle at the seed status (`Proposed`, `Draft`, `open`), the shape issue 0047 first proposed. It is the zero-risk option — an unaccepted record has no citations to break — but it answers only the cheap half of the problem and leaves `supersede` as the remedy for a typo in an accepted record, which is what the issue set out to remove.

Rejected: retitling in place without renaming the file, freezing the slug as a stable identifier. No link can break, including outside the bundle, but it buys that by making filename-versus-title divergence permanent and by asking the gate to tolerate exactly the mismatch the gate exists to catch.

Rejected: rewriting references outside the bundle. The tool would have to claim the whole repository as its edit surface, and a published URL stays broken anyway. A report at the boundary of what the tool owns is honest; a partial rewrite that looks total is not.

## Consequences

**Easier / gained:**
- A wrong title is a one-line fix on any live record, and `supersede` goes back to meaning a changed decision.
- The bundle has no broken-link window: the rename and the link rewrite land in the same transaction, so `check` passes without a follow-up pass.
- `check` now names every record whose heading and title disagree, making visible the hand-edit path the CLI-first rule already forbids.

**Harder / accepted trade-offs:**
- References outside the bundle break on a retitle of a cited record. The verb names them; the author repairs them.
- `DocStore` grows a fourth method, and every test double in the workspace implements it.
- A retitle rewrites other records' bytes, which no `set` key did before; the blast radius of the verb is now the bundle, not one file.

**Follow-ups:**
- None. The outside-bundle report is part of this change, not a deferred one.

## Verification

**Implementation impact:** `living-docs-core/src/store.rs` (the `rename` port), `fs-store/src/lib.rs`, `living-docs-core/src/commands/set.rs` and a new `set/retitle.rs`, `living-docs-core/src/check/records.rs` (heading-versus-title finding), and every `DocStore` test double.

**Verification criteria:**
- `set <ref> title "<new>"` on an `Accepted` ADR rewrites frontmatter, heading and filename, rewrites every in-bundle link that pointed at the old path, and `check` passes with no `broken link` finding.
- The same call on a record whose status is terminal for its type, or `Superseded`, exits non-zero and names `supersede`; nothing is written.
- A new title whose slug already exists is refused before any write.
- A record whose heading disagrees with its `title` is a `check` advisory, and the bundle's violation list stays empty.

# References

[1] [Issue 0047 — set accepts title so a Proposed record can be retitled](../issues/0047-set-accepts-title-so-a-live-record-can-be-retitled-without-hand-editing-frontmatter.md)
