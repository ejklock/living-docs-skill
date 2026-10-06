# PRD Conventions

A PRD is **optional**. Write one when a feature has a "who asked / what is out of scope / what success looks like" worth pinning down before the work starts. A PRD without that is just a large issue — keep it an issue. A PRD answers *what and why*; the ADRs and issues it spawns answer *how*.

## Format

Each PRD is an **OKF concept** (`type: PRD`) — see the `okf-knowledge-format` skill. `status` (`Draft` | `Accepted` | `Implemented` | `Superseded`) lives in the frontmatter, not a body line. See `templates/prd.md`. Required sections: `Problem / Motivation` and `Non-goals`; `check` enforces them on live records (a Deprecated or Superseded PRD is not held to them). Core sections:

- **Problem / Motivation** — the user or system pain. Lead with the problem, not the solution. If you can't state the problem without naming a solution, grill it first (`grill-me`).
- **Who asked** — the stakeholder or need driving this. A PRD with no identifiable requester is a solution looking for a problem.
- **Goals** — what success looks like, as outcomes (not tasks).
- **Non-goals** — what this explicitly does *not* cover. The most valuable section: it bounds scope and prevents creep.
- **Requirements** — testable statements of what the system must do. Each must be falsifiable: "Search returns in <200ms at p95", not "search should be fast". A quality requirement (performance, availability, scale, security) states its measure and how it is verified.
- **Acceptance criteria** — observable conditions that prove the requirement is met.
- **Open questions** — unresolved decisions, each headed toward an ADR.

## Relationship to the constitution

A PRD sits **under** the constitution — it specifies a feature within the product's established principles and constraints. A PRD never replaces or overrides the constitution. If a PRD requires a change at constitution level, resolve that separately before accepting the PRD.

## Rules

1. **One capability per PRD.** Number sequentially: `docs/prd/NNNN-slug.md`. Index in `docs/prd/index.md` (OKF reserved listing, no frontmatter).
2. **Problem before solution.** A PRD that opens with the implementation has skipped the thinking. Restate the underlying problem first.
3. **Non-goals are mandatory.** An empty Non-goals section means scope is undefined. Name at least what tempting-but-excluded things are out.
4. **Requirements are testable and measurable.** "The system should be fast" is not a requirement; "Search returns in <200ms at p95" is. A quality requirement without a way to verify it is a vibe — state the instrument (a load test, a CI floor, a security check) or, only for the genuine residue, explicit inspection.
5. **Success metrics are mandatory.** State how success will be measured after delivery — quantified outcomes (not task completion) that would confirm the problem is solved.
6. **Append-only once accepted.** A PRD under active design is editable. Once accepted and being implemented, changes are recorded as amendments or new ADRs — not silent edits to the requirements.
7. **PRDs spawn issues.** Each requirement becomes one or more issues (see `rules/issue-workflow.md`). The PRD links to them; the issues link back to the PRD.
8. **Open questions resolve into ADRs.** When an open question is answered with a load-bearing rationale expensive to reverse, write an ADR and link it from the PRD. A cheap resolution goes in the issue that carries the work.

## Anti-patterns

- A PRD that is a task list. Tasks are issues; the PRD is the spec they serve.
- A PRD written for a change that had no "who asked / out of scope" to pin — it should have been an issue.
- No acceptance criteria — then "done" is a matter of opinion.
- Editing accepted requirements in place when scope changes — amend or supersede so the history of what was agreed survives.
