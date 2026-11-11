---
type: ADR
title: Teach record types by a decision test and a counterexample leak table, delivered in the template slot, with a LEAK advisory for the detectable cases
description: each record type gets one decision test and a leak table of counterexamples in the served topics; the per-slot constraint travels in the judgment slot brief emits; check adds a regex/structure LEAK advisory for the five detectable leaks, exit code unchanged
owner: Evaldo Klock
status: Deprecated
timestamp: 2026-09-11T12:52:15Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0054. Teach record types by a decision test and a counterexample leak table, with a LEAK advisory

## Context

The skill already defines each record type and each template carries judgment slots, yet content leaks across types: an ADR that carries target behavior (BDR content), a test-count JSON blob (issue/research content), or an issue that says "Needs an ADR (how) and a BDR (behavior)" — deferring the decision instead of taking it. Definitions did not prevent this: the agent reads "an ADR captures one decision" and still writes behavior and evidence in, because nothing tells it what a decision is *not*, and the guidance arrives as prose far from the moment of writing.

## Decision

We will teach the boundaries by test and counterexample, delivered at the point of use, and instrument the detectable cases:

- **One decision test per type** (`doc-trail.md`): a single question whose empty answer means the record belongs elsewhere — ADR "which alternative was rejected?", BDR "which test fails if this breaks?", PRD "who asked, what is out of scope?", issue "what is the diff?", research "which external source?".
- **A leak table of counterexamples** (`doc-trail.md`): each row is content that leaked, the type it landed in, and where it belongs — agents learn boundaries from counterexamples better than from definitions.
- **Guidance in the slot, not a rules file.** `brief` emits a `<!-- hint: … -->` line beside each `<!-- judgment: … -->` marker (e.g. context → "the forces…; <= 80 words; no solution here"), so the constraint appears where the agent writes and disappears once filled. The always-loaded SKILL.md stub does not grow.
- **A `LEAK` advisory in `check`** (regex/structure, no LLM): a Given/When/Then scenario in an ADR/PRD, a JSON/data fenced block in an ADR outside `## Verification`, an unfilled `{{PLACEHOLDER}}` in any record, a BDR scenario with no `Proves:` line, and "Needs an ADR/BDR" in an issue. Exit code unchanged; the undetectable leaks stay in the table and never become a hard stop.

## Consequences

**Easier / gained:**
- The agent gets a per-type "is this the right record" test and point-of-use constraints, and the mechanical leaks are caught before review.

**Harder / accepted trade-offs:**
- The `LEAK` fenced-block detector targets JSON/data blocks, not every code fence, because a legitimate config snippet in a Consequences section is not a leak — the detector under-reports to stay trustworthy.

**Follow-ups:**
- Tune the leak detectors' phrasings and scope as false-positive rates surface on real corpora.

## Verification

**Implementation impact:** `living-docs-core/src/check/leak.rs` (new), `living-docs-core/src/check/mod.rs`, `living-docs-core/src/commands/brief/constraints.rs` (new), `living-docs-core/src/commands/brief.rs`, `skills/living-docs/rules/doc-trail.md`.

**Verification criteria:**
- `check` prints a `LEAK` advisory on a fixture for each of the five detectable cases and exits 0; `check docs` over this repo stays green.
- `brief` output carries a `<!-- hint: … -->` line for the mapped judgment slots; the decision test and leak tables are served by `--topic doc-trail`.
