# CLAUDE.md — Hard Rules Template

Copy this block into the project's `CLAUDE.md` and fill in the `<placeholders>`. Adapt the table of locations to match the project's actual directory structure.

---

## Living Docs

Living Docs is a **decision log with a gate**. It records the engineering decisions a future reader would pay to rediscover, and refuses to let a record exist unindexed, untyped, or silently rewritten. It is not a per-layer paperwork requirement.

**The one rule that decides whether to write a record:**

> Write an **ADR** when a future reader would pay to rediscover *why* you chose this over the alternatives — i.e. the decision is expensive to reverse. Otherwise put the choice in the **issue** that carries the work. When in doubt, it is an issue, not an ADR.

Most changes need no new decision record. A record is earned by materiality (a choice that is expensive to reverse), never by the fact that a change touched structure or behavior. Do not manufacture a record per layer.

Load-bearing decisions are always confirmed with you before an ADR is recorded; the agent never back-fills a decision by inference.

## Doc-trail

Records exist so a decision can be traced from its rationale to the code:

```
constitution → PRD (optional) → ADR → issues → code
```

Only what the change earns: a routine change is just an issue and code. A material decision earns an ADR in the same change. A product/feature spec earns a PRD when there is a "who asked / what is out of scope" worth pinning; a PRD without that is just a large issue.

## Locations

Adapt this table to the project's actual paths before committing the rules.

| Artifact | Location |
|---|---|
| Constitution | `docs/constitution.md` |
| PRDs | `docs/prd/` |
| ADRs | `docs/adr/` |
| Issues | `docs/issues/` |
| Research | `docs/research/` |

---

## Hard rules

### 1. Docs-first for material decisions

A change that makes a decision expensive to reverse — an architecture, dependency, schema, or public-contract choice — ships its ADR in the same PR. A routine change shipping without a new record is complete; a material decision shipping without its ADR is not.

### 2. Diagrams are always Mermaid

All diagrams in documentation must be Mermaid. Existing ASCII diagrams are converted whenever their containing doc is touched. Never introduce image-based or ASCII diagrams.

### 3. Semantic doc groups with OKF index files

Every directory of documentation is a semantic group and must contain an `index.md` (the OKF reserved listing — no frontmatter, except the bundle-root `docs/index.md`, which declares `okf_version: "0.1"`). Every concept document opens with OKF frontmatter carrying a non-empty `type`; `status` and supersession live in frontmatter, never a body line. Every new document is linked from its group's `index.md` with bundle-relative (`/…`) links, and new groups are linked from the root docs index. No orphan documents. See the `okf-knowledge-format` skill.

### 4. The record types

Four types answer four distinct questions. If a candidate record does not answer one of these, it is not that type — do not open it:

- **ADR** — *what did we choose, what did we reject, and why?* A decision expensive to reverse, with its alternatives. `docs/adr/NNNN-slug.md`.
- **Issue** — *what is the change, and how do we know it is done?* The unit of work; it carries any cheap-to-reverse choice inline.
- **Research** — *what does external evidence say?* Dated evaluation of outside sources, in the OKF format (see the `research-artifacts` skill).
- **Constitution** — *what never changes here?* The singleton of scope and non-negotiables.

A **PRD** is optional: a product/feature spec with who-asked and out-of-scope. Behavior is specified by tests, not by a separate record type. A test-strategy *decision* is an ADR `tags: [testing]`.

### 5. Issues local-first

Draft the issue as `docs/issues/NNNN-slug.md` first, linked from the issues index. Launch on the tracker, stripping the OKF frontmatter so only the body is sent. Backfill the tracker number into the issue's frontmatter (`tracker`) and the index. The local file is the trace; the tracker is execution state.

### 6. No comments in code

Self-documenting names, small single-purpose functions, and extracted variables replace comments. A comment is permitted only for a constraint the code cannot express — a non-obvious external contract, a deliberate workaround with its reason. Never comment to narrate what the code does, restate history, or address a reviewer. No commented-out code. Never reference a project doc artifact from a docblock or comment — no ADR, PRD, issue, constitution article, research note, or delivery-slice name or number (for example `see ADR 0046` or `slice R2`); doc numbers change when records are superseded, so the reference rots — state the invariant itself and let the docs carry the numbering. External specification identifiers (an RFC, a CVE) are stable and remain allowed.

### 7. All internal artifacts in English

Code, documentation, commit messages, ADRs, and issue drafts are written in English. Conversation language follows the user.

### 8. Generated artifact names describe what they do

Migrations, scripts, and auto-named artifacts use descriptive names. For example: `--name <what_it_does>`, never auto-generated whimsical names. The name must let a future reader understand the artifact's purpose without opening it.

### 9. Quality gates — all must pass before merge

All of the following must pass on every PR. No exceptions, no deferrals.

| Gate | Command |
|---|---|
| Tests | `<test command>` |
| Type checking | `<typecheck command>` |
| Lint at zero warnings | `<lint command at zero warnings>` |
| Mutation testing (changed code, per file) | `<mutation testing >= N% on changed code, per file>` |

The docs-update rule from rule 1 is also a quality gate: a PR failing `living-docs check` does not merge.

### 10. Author docs through the living-docs CLI — never hand-do deterministic steps

The dividing line is determinism: any documentation step with a single correct output given its inputs goes through the `living-docs` CLI; only the judgment prose (the "why") is authored by hand, directly in the file.

- Use the CLI verb for every mechanical step: `living-docs new <type> "<title>"` (number + frontmatter + skeleton), `living-docs set <ref> <key> <value>` (sets `status`/`description`/`owner`), `living-docs supersede <old> <new>` (wires `supersedes`/`superseded_by` + status on both records), `living-docs index [type]` (regenerates the index), `living-docs check` (the doc-gate, must pass).
- Write the body prose directly — there is no paragraph-editing verb, because wrapping a text edit in the CLI adds no determinism. Editing the body is a normal edit; hand-numbering a doc, hand-writing frontmatter, hand-maintaining an index row, or hand-wiring supersede links is a process error. Each paragraph is ONE line — never hard-wrap prose at a fixed column (`living-docs fmt` unwraps it).
- When a deterministic frontmatter mutation has no verb yet and it keeps being done by hand, harden it into a `living-docs set` key rather than normalizing the hand-edit.
- A record whose body opens with a `> **SUPERSEDED …**` or `> **DEPRECATED …**` callout is history. Follow the successor link, or discard the record if it has none; never plan on it. Read `living-docs read` for what is in force. The callout is CLI-written by `supersede`, `set`, or `fmt` — never by hand. `fmt` also seeds a missing timestamp (from the adding commit) and a missing required owner (from git config) and aligns the title heading with the frontmatter title; it changes no other prose.

**Write ONLY the body below the closing `---`.** Numbering, frontmatter (`type`, `title`, `status`, `supersedes`, `superseded_by`, `timestamp`), and index rows are CLI-owned; `description` and `tags` are yours.

This rule is enforced by one gate, not prose: `living-docs check` runs at commit and in CI, and a hand-written record fails there in the session that authored it. Wire it once per project with:

- **`living-docs install hooks [--dir <path>] [--docs-dir <bundle>] [--dry-run]`** (every harness): materializes the session-teaching script, wires it into `.claude/settings.json`, and installs a `pre-commit` hook running `living-docs check <bundle>`. Remove everything it wrote with the sibling `living-docs uninstall hooks [--dir <path>] [--dry-run]`.
- **Knob:** `LIVING_DOCS_BUNDLE=<dir>` (docs bundle scope).
