# PRD Conventions

A Product Requirements Document specifies **one** feature or capability: the problem it solves, who it's for, what "done" means, and what is explicitly out of scope. A PRD answers *what and why*; the ADRs and issues it spawns answer *how*.

## Format

Each PRD is an **OKF concept** (`type: PRD`) — see the `okf-knowledge-format` skill. `status` (`Draft` | `Accepted` | `Implemented` | `Superseded`) lives in the frontmatter, not a body line. See `templates/prd.md`. Core sections:

- **Problem / Motivation** — the user or system pain. Lead with the problem, not the solution. If you can't state the problem without naming a solution, grill it first (`grill-me`).
- **Goals** — what success looks like, as outcomes (not tasks).
- **Non-goals** — what this explicitly does *not* cover. The most valuable section: it bounds scope and prevents creep.
- **Requirements** — numbered, testable statements. Each maps to acceptance criteria.
- **Acceptance criteria** — observable conditions that prove the requirement is met.
- **Open questions** — unresolved decisions, each ideally headed toward an ADR.
- **Decision log** — links to the ADRs that resolved the open questions.

## Relationship to the constitution

A PRD sits **under** the constitution — it specifies a feature within the product's established principles and constraints. A PRD never replaces or overrides the constitution. If a PRD requires a change at constitution level, resolve that separately before accepting the PRD.

## Rules

1. **One capability per PRD.** Number sequentially: `docs/prd/NNNN-slug.md`. Index in `docs/prd/index.md` (OKF reserved listing, no frontmatter).
2. **Problem before solution.** A PRD that opens with the implementation has skipped the thinking. Restate the underlying problem first.
3. **Non-goals are mandatory.** An empty Non-goals section means scope is undefined. Name at least what tempting-but-excluded things are out.
4. **Requirements are testable.** "The system should be fast" is not a requirement. "Search returns in <200ms at p95" is.
5. **Success metrics are mandatory.** Each PRD must state how success will be measured after delivery — quantified outcomes (not task completion) that would confirm the problem is solved.
6. **Append-only once accepted.** A PRD under active design is editable. Once accepted and being implemented, changes are recorded as amendments or new ADRs — not silent edits to the requirements.
7. **PRDs spawn issues.** Each requirement becomes one or more issues (see `rules/issue-workflow.md`). The PRD links to them; the issues link back to the PRD.
8. **Open questions resolve into ADRs (how) or BDRs (behavior).** When an open question is answered with a load-bearing rationale about architecture, write an ADR and link it from the decision log. When the question resolves observable behavior — what the system must do, with Given/When/Then scenarios — write or amend a BDR instead. Both artifact types are spawned by the PRD and link back to it.

## Anti-patterns

- A PRD that is a task list. Tasks are issues; the PRD is the spec they serve.
- No acceptance criteria — then "done" is a matter of opinion.
- Editing accepted requirements in place when scope changes — amend or supersede so the history of what was agreed survives.
