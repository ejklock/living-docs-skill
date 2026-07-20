# CLAUDE.md — Hard Rules Template

Copy this block into the project's `CLAUDE.md` and fill in the `<placeholders>`. Adapt the table of locations to match the project's actual directory structure.

---

## Living Docs

```
enforcement: strict   # strict | guided | lite
onboarded: <YYYY-MM-DD>
```

The enforcement mode is set once, the first time living-docs runs in this project, and persisted here. `strict` (default) = the doc trail below is mandatory and the agent refuses a structural/behavioral task that ships without its doc; `guided` = the agent asks before skipping a step; `lite` = only the five core invariants are enforced, the doc trail is advisory. To change modes, edit this block. See the living-docs skill → *Enforcement modes*. The mode controls *completion enforcement* only — load-bearing decisions are always grilled and confirmed with you before an ADR/BDR is recorded; the agent never back-fills a decision by inference, in any mode.

## Doc-trail

Every change follows this chain:

```
constitution → PRD → ADR (how) + BDR (behavior) → issues → code
```

No code change skips a link. A PR that changes behavior without a BDR, or architecture without an ADR, is incomplete (subject to the enforcement mode above).

## Locations

Adapt this table to the project's actual paths before committing the rules.

| Artifact | Location |
|---|---|
| Constitution | `docs/constitution/` |
| PRDs | `docs/prd/` |
| ADRs | `docs/decisions/` |
| BDRs | `docs/behavior/` |
| Research | `docs/research/` |
| Issue drafts | `docs/issues/drafts/` |

---

## Hard rules

### 1. Docs-first change policy

Any change that alters behavior, architecture, flows, endpoints, or schema MUST update the corresponding docs in the same PR. A PR that changes code without updating its affected docs is incomplete and must not be merged.

### 2. Diagrams are always Mermaid

All diagrams in documentation must be Mermaid. Existing ASCII diagrams are converted whenever their containing doc is touched. Never introduce image-based or ASCII diagrams.

### 3. Semantic doc groups with OKF index files

Every directory of documentation is a semantic group and must contain an `index.md` (the OKF reserved listing — no frontmatter, except the bundle-root `docs/index.md`, which declares `okf_version: "0.1"`). Every concept document opens with OKF frontmatter carrying a non-empty `type`; `status` and supersession live in frontmatter, never a body line. Every new document is linked from its group's `index.md` with bundle-relative (`/…`) links, and new groups are linked from the root docs index. No orphan documents. See the `okf-knowledge-format` skill.

### 4. Architectural decisions require an ADR; expected behavior requires a BDR

- A decision that changes structure, dependencies, or technical approach → write an ADR (`docs/decisions/NNNN-slug.md`).
- A decision that defines or changes what the system must observably do → write or amend a BDR (`docs/behavior/NNNN-slug.md`). Every BDR carries a Mermaid diagram, a textual description, a Contract section (public signatures + agent tool schemas, observable-only), numbered Given/When/Then scenarios, and a Test Design matrix.
- Research informing decisions goes in `docs/research/` in the OKF format (dated session directory with a `report.md` concept and a `references.md` registry; see the `research-artifacts` skill).

### 5. Issues local-first

Draft the issue as `docs/issues/drafts/NNNN-slug.md` first, linked from the drafts index. Launch on the tracker, stripping the OKF frontmatter so only the body is sent. Backfill the tracker number into the issue's frontmatter (`tracker`) and the index. The local file is the trace; the tracker is execution state.

### 6. No comments in code

Self-documenting names, small single-purpose functions, and extracted variables replace comments. A comment is permitted only for a constraint the code cannot express — a non-obvious external contract, a deliberate workaround with its reason. Never comment to narrate what the code does, restate history, or address a reviewer. No commented-out code.

### 7. All internal artifacts in English

Code, documentation, commit messages, ADRs, BDRs, and issue drafts are written in English. Conversation language follows the user.

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

The docs-update rule from rule 1 is also a quality gate: a PR failing the docs check does not merge.

### 10. Author docs through the living-docs CLI — never hand-do deterministic steps

The dividing line is determinism: any documentation step with a single correct output given its inputs goes through the `living-docs` CLI; only the judgment prose (the "why") is authored by hand, directly in the file.

- Use the CLI verb for every mechanical step: `living-docs new <type> "<title>"` (number + frontmatter + skeleton), `living-docs supersede <old> <new>` (wires `supersedes`/`superseded_by` + status on both records), `living-docs index [type]` (regenerates the index), `living-docs check` (the doc-gate, must pass), `living-docs export`/`brief` (byte-stable materialization / pre-filled scaffold).
- Write the body prose directly — there is no paragraph-editing verb, because wrapping a text edit in the CLI adds no determinism. Editing the body is a normal edit; hand-numbering a doc, hand-writing frontmatter, hand-maintaining an index row, or hand-wiring supersede links is a process error.
- When a deterministic frontmatter mutation has no verb yet (e.g. set status, add a tag) and it keeps being done by hand, harden it into a new CLI verb rather than normalizing the hand-edit.
