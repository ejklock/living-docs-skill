---
type: Issue
title: "Doc-code pairing for living-docs: commit trailers, covers-based drift detection, and executable acceptance"
description: Make "what was planned is what is implemented" mechanically checkable by layering three guarantees onto the tracker -- commit trailers, covers-based drift detection, and executable acceptance -- each cheap enough to adopt independently and each feeding evidence to the next.
status: closed
timestamp: 2026-08-03T13:53:21Z
---

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly open | in-progress | closed. `living-docs supersede` sets Superseded on
     this issue -- never set it by hand -- when a later issue replaces it. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Doc-code pairing for living-docs: commit trailers, covers-based drift detection, and executable acceptance

> **CLOSED OBSOLETE 2026-09-22.** All three layers below rest on premises
> [ADR 0059](../adr/0059-cut-living-docs-to-the-authoring-core-remove-the-database-read-model-web-front-public-export-migrate-json-authoring-and-the-write-gate-hook.md)
> removed. Layers 2 (covers-glob drift) and 3 (executable acceptance) need a
> persistent projection to hold the hash pairs and the per-criterion results;
> the db-store read-model that would hold them was deleted, and
> [ADR 0031](../adr/0031-the-knowledge-graph-is-a-typed-bi-temporal-edge-set-in-the-existing-relational-store.md),
> which specified that substrate, is Deprecated with no successor. Layer 1
> (commit trailers) asks for three new verbs — `commits`, `verify`,
> `reconcile` — against the ten-verb contract of
> [ADR 0060](../adr/0060-the-cli-surface-is-a-contract-ten-intention-named-verbs-one-output-mode-per-stream-documented-exit-codes-and-help-written-for-an-agent.md).
> Doc-code lineage re-enters, if it does, as a fresh record written from the
> post-0059 constraint: the `.md` tree in git, ten verbs, no read-model. The
> only slice that shipped from this issue was the artifact file manifest
> (issue 0032), itself retired with the artifact doc type.

Filed downstream in `ai-configs` as its issue 0040, against this CLI as the
implementation target -- ported here verbatim (adapted for local framing)
since this repo is where the work actually lands. Nothing today guarantees
that a tracker record and the code it governs stay in sync: a downstream
consumer hit this in practice when a record's Plan said "single slice" but
execution took three, and only a hand-maintained reconciliation line closed
the gap. That convention is a manual patch over a structural hole: the
doc-to-code link is prose, so drift in either direction (code changed, doc
stale; plan changed, code pending) is invisible until a human notices.

This issue specifies three layers of doc-code pairing, ordered from cheapest
to strongest guarantee. Each layer is independently adoptable and each
produces the evidence the next layer consumes.

Key design stance: **proximity proves correlation, not conformance.**
Layers 1-2 prove that doc and code *moved together*; only Layer 3
(executable acceptance) proves the code *does what the doc says*. A
constraint without an instrument is a vibe -- the layers exist to attach
instruments of increasing strength.

### Scope

**Layer 1 -- commit trailers as foreign keys (mechanical link).**

- Convention: every commit implementing a record carries a
  `Living-Doc: NNNN` trailer (same mechanics as `Co-Authored-By:`).
- CLI additions:
  - `living-docs commits NNNN` -- list linked commits
    (`git log --grep '^Living-Doc: NNNN$'` over trailers).
  - `living-docs verify` -- reciprocal checks: (a) every `Accepted`/closed
    record has >= 1 linked commit ("accepted on paper" detector); (b) every
    trailer resolves to an existing record (orphan-work detector).
  - `living-docs reconcile NNNN` -- derives an "Executed as:" line from the
    linked-commit list and writes it as CLI-owned metadata, replacing any
    hand-maintained reconciliation prose.
  - **Assisted binding, not just manual convention:** a hand-typed trailer
    is a convention a human (or an agent that forgets) can simply omit,
    which undermines Layer 1's proof value at the source. Two independent,
    complementary assists, neither mandatory, neither blocking a commit:
    - **Agent-assisted (skill-level, zero CLI code):** the living-docs
      skill's own instructions direct any agent committing on behalf of a
      tracked record (this org already carries the identical discipline
      for ticket-ID trailers, e.g. `[PA-4596]`) to add `Living-Doc: NNNN`
      itself, inferred from the task/record it was asked to implement --
      the agent already tracks that context through its own pipeline, so
      this is a documentation change to the skill, not a CLI feature.
    - **Hook-assisted (CLI-level, structural):** `living-docs hooks
      install` (the existing deterministic-hook channel, ADR 0023) ships
      an opt-in `prepare-commit-msg` hook for human-authored commits with
      no agent present: if staged files match exactly one record's
      `covers:` globs (Layer 2), the hook injects that record's trailer;
      on zero or multiple matches it does nothing -- it never blocks the
      commit or guesses under ambiguity.
    - `living-docs commit --for NNNN` is the explicit escape hatch either
      assist can fall back to (scripts, CI, or a commit neither could
      infer): plain `git commit` with the trailer appended.
    - All three are assistive only -- `verify`/`commits` read whatever
      trailer ends up in the commit, however it got there.
  - **Adoption in an existing bundle:** check (a) cannot apply retroactively
    -- every project adopting this CLI already has accepted/closed records
    with no `Living-Doc:` trailer in their commit history, and rewriting
    published commit messages to backfill trailers is off the table. `new`
    writes a one-time CLI-owned cutover marker (e.g. `verify_since:
    <commit-sha>` in a bundle-level config, set by a `living-docs verify
    --adopt` step run once per project) the moment a project turns Layer 1
    on. Check (a) only holds records that transition to `Accepted`/closed
    *after* that marker to the linked-commit guarantee; everything already
    accepted/closed at adoption time is grandfathered in, silently, with no
    backfill required. Check (b) (orphan trailers) has no such problem --
    it only ever looks at trailers that exist, so it needs no cutover.

**Layer 2 -- covers-glob drift detection (bidirectional staleness).**

- New CLI-owned frontmatter field `covers:` -- a list of path globs the
  record governs (a CODEOWNERS-for-docs), set via
  `living-docs covers NNNN <glob>...`, never hand-edited.
- `living-docs drift` (runnable in pre-commit or CI) detects both
  directions:
  - *code moved, doc did not*: a commit touches covered paths with no
    `Living-Doc:` trailer and no body edit to the record -> warn
    "doc possibly stale".
  - *doc moved, code did not*: record body hash changed with no subsequent
    linked commit -> record enters a "plan ahead of code" state until one
    lands.
- Mechanics: the index stores a hash pair per record -- body content hash +
  git tree hash of the covered paths. `verify`/`drift` compare stored vs
  current: unilateral divergence = drift; bilateral divergence with a
  trailer = normal evolution, pair re-recorded.

**Layer 3 -- executable acceptance (conformance, not correlation).**

- The `### Acceptance` section gains an optional structured form the CLI
  parses (prose bullets remain valid for judgement-only criteria):

  ```yaml
  acceptance:
    - id: AC-1
      text: "verifier fails the ref arm on a missing envelope_sha256"
      verify_by: command
      command: "bash tests/unit/validate-routing-evals.sh"
  ```

- `living-docs check NNNN` runs each `verify_by: command` entry and reports
  pass/fail per AC.
- `living-docs status NNNN <closed-equivalent>` can then demand evidence:
  refuse the transition while a command-verifiable AC fails (opt-out flag
  for documented exceptions), and record the check results in CLI-owned
  frontmatter at transition time. "Planned == implemented" stops being
  faith and becomes an exit code.
- Grammar deliberately mirrors the `verify_by` vocabulary already used by
  agent-pipeline TaskEnvelopes elsewhere in this org (`command | test |
  inspection`) so records translate directly into pipeline acceptance
  criteria for any consumer that wants that mapping.

**Ergonomics (small, ride along with any layer):**

- `living-docs list --status <Status>`, `living-docs show NNNN`, and
  `--json` output on read commands (agent consumption -- today agents parse
  markdown by hand).
- `living-docs status NNNN <Status> --evidence <commit|url>` recording
  *why* a transition happened.
- Combined with Layer 1, an in-progress status (this repo already has one
  in the Issue vocabulary -- see ADR 0029) enables `living-docs start NNNN`
  = status flip + branch registration in one step.

**Explicitly KEPT:** the authoring contract (body below the closing `---`
is human/agent-written; frontmatter and indexes stay CLI-owned); prose
acceptance bullets for judgement-only criteria.

**Out of scope:** semantic verification beyond executable ACs (an LLM
judging "does the code match the prose" is a review-time job, not a CLI
job); any change to a downstream consumer's own pipeline envelopes (the
`verify_by` alignment is vocabulary reuse, not a schema change); issue 0021
(`--description` flag) and issue 0022 (placeholder format) -- filed
separately in this repo, independent of this work.

### Acceptance

- A commit carrying `Living-Doc: NNNN` for an existing record is listed by
  `living-docs commits NNNN`; `living-docs verify` exits non-zero when an
  accepted/closed record has zero linked commits or a trailer references a
  nonexistent record.
- `living-docs reconcile NNNN` writes an "Executed as:" summary derived
  from linked commits, and the result is CLI-owned (hand-edits blocked like
  other frontmatter).
- With a `covers:` glob set, `living-docs drift` flags a commit touching
  covered paths that carries no trailer and no record edit; and flags a
  record body edit with no subsequent linked commit -- both directions
  demonstrated by test fixtures.
- `living-docs check NNNN` runs structured `verify_by: command` acceptance
  entries and reports per-AC pass/fail; the terminal-status transition
  refuses (absent the opt-out flag) while such an AC fails.
- Prose-only records remain fully valid: every existing record in this repo
  passes `verify`/`drift`/`check` untouched (Layer 0 compatibility).
- Adopting Layer 1 on a project with pre-existing accepted/closed records
  does not fail `verify`: `living-docs verify --adopt` records the cutover
  once, and every record accepted/closed before it is exempt from the
  linked-commit check permanently, with no hand-editing of history.
- Consumers outside this repo (e.g. `ai-configs`) can adopt Layer 1 the
  moment `verify` exists, with no CLI change required to start emitting
  trailers.

### Calibration-pending (unverified facts)

- Whether trailer greps stay fast enough on large histories without an
  index cache (`git log --grep` is linear; may need a cached map keyed by
  record number).
- False-positive rate of `drift` on covers globs in practice (too-broad
  globs will nag; the fix is narrowing globs, but the ergonomics need real
  usage to calibrate).
- Whether `status --evidence` and Layer 3's transition gate should be
  hard-fail or warn-by-default in 0.x (adoption friction vs guarantee
  strength).
- Layers 2 and 3 do not share Layer 1's retroactivity problem: `covers` and
  structured `acceptance:` blocks are per-record fields nobody sets until
  an owner opts a record in, so an un-migrated existing record is simply
  never checked rather than failing -- worth confirming this holds once
  `drift`/`check` are actually built, not just assumed here.

### Plan

Layered so each ships alone; each layer's data calibrates the next.

1. **Layer 1** -- trailer convention + `commits`/`verify`/`reconcile`.
   Adoptable in a day; immediately supersedes any hand-maintained
   reconciliation convention downstream.
2. **Layer 2** -- `covers` field + hash-pair index + `drift`. Catches the
   large majority of forget-to-update cases at low cost.
3. **Layer 3** -- structured acceptance + `check` + evidence-gated terminal
   status. The endgame guarantee; requires the discipline of
   command-verifiable ACs.
4. Ergonomics (`list`/`show`/`--json`/`--evidence`) ride along with
   whichever layer touches the relevant command first.
