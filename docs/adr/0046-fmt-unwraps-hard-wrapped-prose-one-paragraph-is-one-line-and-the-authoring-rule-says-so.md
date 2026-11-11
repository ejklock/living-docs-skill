---
type: ADR
title: "fmt unwraps hard-wrapped prose: one paragraph is one line, and the authoring rule says so"
description: living-docs fmt joins the lines of each prose block into one line; code, tables, headings, comments, and hard breaks stay untouched; the authoring topics state the rule.
owner: Evaldo Klock
status: Superseded
superseded_by: 0047
timestamp: 2026-09-01T19:57:47Z
---

> **SUPERSEDED — do not act on this record.** Replaced by [0047](0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md). Run `living-docs read records` for what is in force.

# 0046. fmt unwraps hard-wrapped prose: one paragraph is one line, and the authoring rule says so

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

Authoring models hard-wrap markdown prose at ~80 columns by habit. No rule in this repo asks for it: `living-docs fmt` (ADR 0019) canonicalizes frontmatter only, the canonical check compares frontmatter only, and the provenance seal (ADR 0039) covers six frontmatter keys. The result is a body that renders correctly but reads badly in every soft-wrapping surface (diff views, editors, the web front): lines break at a fixed column, then break again at the viewport. A CLI-generated index row sits on one line next to hand-wrapped prose, so one record mixes two conventions.

Two forces meet here. The determinism boundary (CLAUDE.md rule 4) says the tool never writes rationale; joining the lines of a paragraph changes no word and no meaning, so it is a mechanical normalization, like collapsing the frontmatter gap. The authoring rule (`procedure.md`, "Write the body prose directly") says there is no paragraph-editing verb because a text edit adds no determinism; unwrapping is not an edit of the text, it is a normalization of its layout, and it has exactly one correct output.

Alternatives considered:

- **Instruction only** (tell the model to write one line per paragraph): it prevents new wrapped records but never repairs the existing corpus, and model habits regress between sessions.
- **A `check` error on wrapped prose**: it fails every brownfield bundle on adoption and treats a layout habit as a fact violation; `check` enforces the fact contract (`type`, `status`, links), not layout.
- **An external formatter (markdownlint MD013, prettier `proseWrap: never`)**: a second toolchain, configured per project, outside the deterministic layer the CLI owns and outside the doc-gate hook.

## Decision

We will make `living-docs fmt` unwrap hard-wrapped prose in every record body it already rewrites, and we will state the one-paragraph-one-line rule in the authoring topics the model loads before writing.

The body pass joins the lines of each prose block into one line with single spaces. A prose block is a paragraph, or a list item together with its indented continuation lines. The pass leaves untouched: the frontmatter, fenced code blocks (```` ``` ```` and `~~~`), indented code blocks that open after a blank line, tables, headings, blockquotes, thematic breaks and setext underlines, HTML blocks and multi-line HTML comments, link-reference definitions, and any line that ends with a markdown hard-break marker (two trailing spaces or a backslash). The pass is idempotent: the second run over its own output rewrites nothing. `check` stays frontmatter-only; hard-wrapped prose is a `fmt` diff, never a `check` finding.

The authoring rule lives in `skills/living-docs/rules/procedure.md` (the "Write the body prose directly" bullet), and the project-guide template and the research rules point at it. Instruction and instrument ship together: the rule prevents, `fmt` repairs.

## Consequences

**Easier / gained:**
- One layout for every record: CLI-generated rows and model-written prose read the same in diff views, editors, and the web front.
- Repairing a legacy bundle is one command (`living-docs fmt`), with no external toolchain.
- Reviewers see a real diff on a prose edit, not a re-wrap of the whole paragraph.

**Harder / accepted trade-offs:**
- The first `fmt` run over an existing bundle rewrites every wrapped record: a large, one-time, content-neutral diff. Run it in its own commit.
- `fmt` now parses block structure. A construct the classifier does not know (an exotic HTML block, a nested table in a list) is treated as prose and joined. The classifier is conservative: any line that opens a known non-prose block, and any line after a hard-break marker, stays as it is.
- Long lines: a paragraph becomes one line of arbitrary length. Line-oriented tools (`grep -n`, blame) point at a paragraph, not a sentence. Accepted: soft-wrap is the reading surface, and one paragraph per line is the git-friendly convention for prose.

**Follow-ups:**
- Run `living-docs fmt` over this repo's `docs/` tree in a separate commit once the verb ships.
- Consider a warn-level `check` finding for wrapped prose only if the corpus regresses after the instruction ships; not now.

## Verification

**Implementation impact:** `living-docs-core/src/commands/fmt.rs` (wires the body pass), `living-docs-core/src/commands/fmt/reflow.rs` (the block classifier and the joiner), `cli/tests/fmt.rs` (integration), `skills/living-docs/rules/procedure.md`, `skills/living-docs/templates/claude-hard-rules.md`, `skills/research-artifacts/rules/rules.md`.

**Verification criteria:**
- A record whose body holds a paragraph wrapped over N lines leaves `fmt` with that paragraph on one line, every word and punctuation mark unchanged, and a single space at each former line break.
- A fenced code block, a table, a heading, a blockquote, a multi-line HTML comment, a link-reference definition, and a line ending in two spaces or a backslash leave `fmt` byte-identical.
- Fitness function: `fmt` run twice over any bundle reports `0 record(s) rewritten.` on the second run, and the bytes are identical (`cli/tests/fmt.rs`, the existing idempotency test extended with a wrapped body).

# References

[1] [CommonMark Spec 0.31.2, §4.5 Fenced code blocks, §4.8 Paragraphs, §5.2 List items, §6.9 Hard line breaks](https://spec.commonmark.org/0.31.2/)

