---
type: ADR
title: "Moved-source review queue: check emits a warn-level finding when a linked record is superseded or demoted"
description: check gains a warn-level MOVED-SOURCE finding when a linked record is superseded or demoted, cleared by linking the successor; the tool builds the review queue, humans judge it
owner: Evaldo Klock
status: Accepted
timestamp: 2026-08-27T19:11:04Z
---

# 0044. Moved-source review queue: check emits a warn-level finding when a linked record is superseded or demoted

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

<!-- The forces at play. What problem forced a decision? What constraints bound it?
     Written so a newcomer understands the pressure without prior knowledge. Link any
     research artifact or PRD/issue that motivates it, bundle-relative:
     [research](/research/NNNN-<slug>.md), [PRD](/prd/NNNN-<slug>.md). -->

A record that links another record inherits meaning from it. When the source is superseded or leaves its accepted state, the dependent record may be silently wrong, and agents consume it at machine speed ([issue 0035](../issues/0035-provenance-review-queue-check-flags-records-whose-referenced-source-was-superseded-or-changed-status.md)). Detecting that a source moved is a diff; judging whether the move invalidates the dependent is human judgment (ADR 0001 determinism boundary). The tool must produce the review queue and stop there.

## Decision

We will add a `MOVED-SOURCE` finding class to `check`, warn-level, with a deterministic clearing rule.

1. **Detection.** For every bundle-relative record link, `check` resolves the target and reports a `MOVED-SOURCE` finding when the target's status is Superseded, or Deprecated/Rejected per that doctype's own status vocabulary (ADR 0029). Links to open/Proposed/Accepted targets and non-record links produce nothing. Broken links keep their existing, separate error class.
2. **Finding shape.** The finding names the dependent record, the moved source, the source's current status, and — when the source is Superseded — its successor from the `superseded_by` link.
3. **Clearing rule.** The finding clears when the dependent record links the successor anywhere in its body, or when the dependent record itself is Superseded/closed. No annotation syntax; the acknowledgment IS the updated link.
4. **Severity.** Warn-level: it never fails the doc-gate. Promotion to error is a later decision, taken only after the existing corpus queue is cleared.

## Consequences

**Easier / gained:**
- Drift between a decision and the records built on it becomes visible at `check` time — a review queue instead of silent staleness.
- The clearing rule is deterministic and needs no new syntax or state file.

**Harder / accepted trade-offs:**
- Linking the successor clears the finding even when a human has not truly re-judged the dependent. Accepted: the tool cannot verify judgment, only its trace.
- Warn-level findings can be ignored. Accepted for now; the promotion decision handles it.

**Follow-ups:**
- Sweep the real `docs/` corpus and clear the initial queue.
- Issue 0037 consumes open `MOVED-SOURCE` findings as a Governed-attribute signal.

## Verification

<!-- OPTIONAL — include when this decision must be honored in code, so the doc closes the
     doc → implement → verify loop an agent (and any review step) can consume. Omit
     for a purely advisory record. Keep criteria checkable, not aspirational.
     Implementation impact: files / modules this decision touches, e.g. `src/store.py`.
     Fitness function: the test / lint / arch-unit assertion that fails if the second
     verification criterion is violated (see `rules/adr-conventions.md` rule 6). -->

**Implementation impact:** `check` link-resolution pass in `living-docs-core`, status-vocabulary lookup via the doctype registry (ADR 0026/0029), CLI `check` output rendering.

**Verification criteria:**
- Given A links B and `supersede B C` runs, `check` reports `MOVED-SOURCE` on A; adding a link to C in A's body clears it; the doc-gate exit code stays zero throughout.
- A link to an Accepted or open target never produces the finding; a broken link still produces the existing error class, not `MOVED-SOURCE`.
- Fitness function: a fixture-corpus test asserts the report → clear cycle and that warn-level findings never change the exit code.

# References

[1] [Making Your Data Ready for Agentic AI — Sadalage & Chandrasekaran](https://martinfowler.com/articles/making-data-ready-for-agentic-ai.html)
