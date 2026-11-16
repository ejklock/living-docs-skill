---
type: Issue
title: The tracker status vocabulary disagrees across the template comment, the validator, and issue 0017 -- and has no in-progress state
description: Unify the tracker status vocabulary — template comment, validator, and docs all say the same thing — and add an in-progress state, closing issue 0017's open half along the way.
status: closed
timestamp: 2026-08-03T12:04:35Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## The tracker status vocabulary disagrees across the template comment, the validator, and issue 0017 — and has no in-progress state

Found dogfooding living-docs as the issue tracker of a heavily-automated repo (agent
orchestration driving `new`/`status`/`index` programmatically).

### Problem

Every generated record's body carries an HTML comment claiming the tracker statuses are
`open | in-progress | closed | superseded`. `living-docs status <NNNN> <Status>` validates
against a *different* set — `Proposed | Accepted | Deprecated` (`Superseded` is rejected;
`supersede` handles it separately). So three vocabularies are in play: the template
comment's, the validator's, and whatever a user infers from either. A user who copies the
comment's dialect gets rejected with exit 2, and is nudged toward hand-editing frontmatter
to work around it — exactly what the CLI-ownership contract forbids.

Separately, `Proposed → Accepted` jumps straight from "not started" to "done". During
execution (multi-hour, multi-slice work) the tracker cannot distinguish in-flight work from
untouched backlog. The only workaround today is a body-level convention (an `**Executing
as:**` line), which frontmatter can't express.

[Issue 0017](0017-bundle-vocabulary-gaps-no-research-doc-type-in-index-and-no-terminal-closed-status-for-issues.md)
already tracks the other open half of this same vocabulary gap: issues have no terminal
`Closed`/`Resolved` state, so a downstream repo had to close a fixed bug as `Deprecated`,
which misstates the outcome. All three gaps — the comment/validator mismatch, the missing
in-progress state, and the missing terminal state for issues — are one decision: what the
tracker status vocabulary is, everywhere, once.

### Scope

Included: deciding the single status vocabulary (an ADR, since it changes validated CLI
behavior and every record template) and updating the validator, the generated template
comment, and any docs that quote the vocabulary to agree with it. Resolves 0017's open half
as part of the same decision.

Explicitly out: the harness-matrix registry question (issue 0019) and the `--description`
flag / placeholder-format gaps, tracked separately.

### Acceptance

- One vocabulary is documented in exactly one place and referenced everywhere else; the
  template's body comment and `living-docs status --help` never disagree.
- The vocabulary expresses an in-flight state distinct from both "not started" and "done".
- Issues have a terminal closed/resolved state distinct from `Deprecated`.
- A record created by `living-docs new` and immediately validated with the comment's own
  vocabulary never produces the exit-2 rejection this issue was filed over.

### Plan

Blocked on a pending ADR that decides the vocabulary. Implementation then touches the
`status` validator, the per-type template body comment, and any prose in `README.md` /
`CONTRIBUTING.md` that quotes the old vocabulary.

### Outcome

Closed: ADR 0029 landed the per-doc-type status vocabulary sourced from one DocTypeSpec field, resolving the disagreement this issue was filed over.
