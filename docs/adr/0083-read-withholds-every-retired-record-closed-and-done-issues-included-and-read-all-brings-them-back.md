---
type: ADR
title: read withholds every retired record, closed and done issues included, and read --all brings them back
description: read withholds every record DocTypeSpec::is_retired reports (closed and done issues included) from all its views and counts them in withheld; read records --all includes them
owner: Evaldo Klock
status: Accepted
timestamp: 2026-11-16T02:33:31Z
---

# 0083. read withholds every retired record, closed and done issues included, and read --all brings them back

## Context

`read` promises the in-force view: active records, supersede chains collapsed. The guides say so in `skills/living-docs/SKILL.md` and `skills/living-docs/rules/semantic-index.md` ("active records only"). The code keeps a narrower rule: `is_in_force` in `living-docs-core/src/commands/effective.rs` withholds only `superseded` and `deprecated`, and ignores the terminal statuses each doc type declares (`DocTypeSpec::terminal_statuses`; an issue's `closed` and `done`). [ADR 0063](0063-authoring-advisories-skip-retired-records.md) already treats those records as retired through `DocTypeSpec::is_retired`.

The cost showed in a downstream bundle (issue 0079): of 366 records, `read` printed 220 closed issues, 56% of its output, and they also swell `--topic` and `--contract`. A closed issue is history; an agent orienting itself pays for it on every read.

## Decision

We will withhold from `read` every record `DocTypeSpec::is_retired` reports, so a closed or done issue leaves the default view, the `--topic`, `--contract` and `--full` views, and counts in `withheld`. Supersede-chain collapsing stays as it is.

We will add `read records --all`, which includes the retired records in any of those views. `--links-to` and `--impact` follow the same rule: by default they list only the records `read` would show, so a closed issue that links the queried record is withheld, and `--all` lists it. This replaces the narrower set [ADR 0074](0074-read-links-to-and-impact-answer-who-depends-on-a-record-from-a-link-graph-built-in-memory-per-run.md) names (no Superseded or Deprecated record) with the plain `read` set it is tied to.

Rejected: withholding without a flag. It is simpler, but a reader looking for a closed issue's decision would have to leave the CLI and open the file.

Rejected: keeping the view and rewording the guides. It costs nothing in code, but it keeps the token cost the issue measured, on the verb agents call first.

## Consequences

**Easier / gained:**
- `read` returns what governs now, at a fraction of the tokens in a bundle with many closed issues.
- One retirement rule (`is_retired`) serves `read`, `set title` and the authoring advisories.

**Harder / accepted trade-offs:**
- A reader who wants a closed issue in the view must pass `--all`.
- `withheld` grows, and a consumer that counted it as superseded records only reads a wider number.

## Verification

**Implementation impact:** `living-docs-core/src/commands/effective.rs` (the in-force rule), the `read records` arguments in `cli/src/` and the MCP read tool, `skills/living-docs/SKILL.md` and `skills/living-docs/rules/semantic-index.md`.

**Verification criteria:**
- `read` on a bundle with one open and one closed issue lists only the open one and reports `withheld: 1`; a `done` issue is withheld the same way.
- `read --all` lists the closed issue.
- The guides' `read` wording names closed and done issues as withheld.
