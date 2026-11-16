---
type: ADR
title: Uniform brace markers for body placeholders
description: Body placeholders in the 6 registered doc-type templates move from prose-punctuated angle brackets to bare {{NAME}} tokens, closing issue 0022's programmatic-edit fragility.
owner: Evaldo Klock
status: Accepted
timestamp: 2026-08-03T16:46:20Z
---

# 0030. Uniform brace markers for body placeholders

<!-- Status lives in frontmatter (`status`), not a body line. Settable values are
     exactly Proposed | Accepted | Deprecated. When superseding a prior ADR, set
     `supersedes` here; `living-docs supersede` sets Superseded on the old record
     -- never set it by hand. -->

## Context

Body placeholders in the ADR/BDR/PRD/Issue/Research/Constitution templates use angle
brackets carrying their own descriptive hint text, e.g. `<the choice, in active voice —
specific and testable>`. Programmatic tooling that fills these placeholders via
exact-string or regex replacement has to match arbitrary embedded prose punctuation
(periods, dashes, commas) verbatim; a single character mismatch breaks the edit — issue
0022 hit this directly, found dogfooding living-docs as the issue tracker of a
heavily-automated repo.

Frontmatter placeholders (`title`, `description`) already have a dedicated fix (issue
0021's `--description` flag and `describe` verb) and are out of scope here. The
`NNNN-<slug>` link-path convention used throughout Related/References sections is a
distinct, already low-risk convention (a single word, never embedded punctuation) and is
also out of scope.

## Decision

We will replace every body placeholder in the 6 registered `DocTypeSpec` templates (ADR,
BDR, PRD, Issue, Research, Constitution) with a bare `{{SCREAMING_SNAKE_NAME}}` token —
brace-delimited, carrying no prose punctuation of its own — following these rules:

1. **Marker syntax:** `{{NAME}}`, `NAME` a short descriptive `SCREAMING_SNAKE_CASE` slug
   (e.g. `{{DECISION}}`, `{{CONTEXT}}`). Names need not be globally unique — a table
   column or a repeated list item (Given/When/Then across multiple scenarios, a generic
   ellipsis cell) may reuse the same name across occurrences; each occurrence in
   isolation is still a clean, punctuation-free token.
2. **Hint relocation:** where the original angle-bracket prose carried guidance already
   implied by the surrounding heading, table header, or rule text, the marker alone
   replaces it — the marker's own name *is* the hint. Where the guidance is not otherwise
   obvious (a formatting constraint, a worked example, a cross-reference format), that
   guidance stays as ordinary prose outside the marker (an adjoining sentence, or the
   template's existing `<!-- -->` comment convention) rather than embedded inside the
   token.
3. **Illustrative examples** (`<e.g. Performance>`-style table cells showing a worked
   example rather than a blank to fill) move their example text outside the marker
   (adjoining prose or a comment); the cell itself becomes a bare marker.
4. Frontmatter placeholders and `NNNN-<slug>` link paths are unchanged (out of scope, per
   Context above).
5. The `# NNNN. <Short decision title>` / `## <Issue title>` heading placeholder (every
   type's H1/H2 title heading, mirroring the frontmatter `title` it duplicates) is also
   unchanged: it carries no embedded prose punctuation — the fragility this ADR fixes
   does not apply to it — and `brief.rs`'s `is_title_heading_placeholder` matches it by a
   literal `"# NNNN. <"` prefix; changing its shape would force a production-logic change
   for no fragility benefit, so it stays out of scope alongside frontmatter.

`brief`'s judgment-slot collapse (`replace_judgment_sections`) already discards a
placeholder's literal text wholesale — it matches on heading lines only, never on body
placeholder content — so migrating a judgment-slot placeholder's prose changes nothing
about `brief`'s behavior or its own test fixtures. The placeholders actually exposed to
the fragility issue 0022 describes are every non-judgment-slot body placeholder (BDR's
Behavior/Contract/Test Design tables, PRD's NFR table, every field when a record is
created via plain `new` without `brief`) plus, for completeness and the fitness
function's uniformity, the judgment-slot placeholders too.

A permanent fitness function (mirroring ADR 0029's per-type template test) asserts every
registered `DocTypeSpec`'s template contains no legacy angle-bracket-with-embedded-space
body placeholder, so a future template edit cannot reintroduce the fragile format.

## Consequences

**Easier / gained:**
- Any exact-string or regex tool locates and fills a placeholder by matching a stable,
  punctuation-free token, regardless of what descriptive prose used to sit inside it.
- The convention is machine-checkable (a template-scanning fitness function), not only
  documented.

**Harder / accepted trade-offs:**
- Every existing template file needs a one-time migration, and every test hardcoding the
  old placeholder text needs updating in the same pass.
- Templates lose some prose self-documentation where a hint moved from inline to a
  nearby comment; authors reading a raw template rely more on section headings.

**Follow-ups:**
- Issue 0022's optional `--body-file` flag (skip placeholder-fill entirely) is not part
  of this decision; it stays a separate, optional follow-up.
- `glossary.md`, `architecture-index.md`, `context-index.md`, and `claude-hard-rules.md`
  are not registered `DocTypeSpec` templates (no NNNN identity, never filled by `new`)
  and are out of scope; they may adopt the same convention later if their own fragility
  surfaces.

## Verification

**Implementation impact:** `skills/living-docs/templates/{adr,bdr,prd,issue,research,constitution}.md`;
`living-docs-core/src/commands/new.rs` (test fixtures only — `new` does not itself
transform body placeholders, and its production code is untouched); `living-docs-core/src/doc_type.rs`
(the fitness function); every test asserting on the old placeholder text
(`cli/tests/new.rs`, `cli/tests/describe.rs`, `cli/tests/index_supersede.rs`). `brief.rs`
needs no change — its judgment-slot collapse matches on heading lines only, never on
placeholder content, and its own test fixtures use a hand-written template constant
independent of the real files (the one exception, its constitution test that loads the
real template, stays valid unchanged since it only asserts the *absence* of specific old
substrings, which remains true regardless of what replaced them).

**Verification criteria:**
- Every registered `DocTypeSpec`'s `template` string contains no bracket-delimited body
  placeholder carrying embedded prose punctuation; every replacement is a bare `{{NAME}}`
  token.
- A fitness-function test in `doc_type.rs` iterates `DOC_TYPES` and fails if any
  template's body still matches the legacy angle-bracket-with-embedded-space shape.
- `cargo test --workspace` stays green throughout every migration slice.

# References

<!-- Optional (OKF §8). External sources backing claims in Context. -->
[1] [Issue 0022](../issues/0022-template-placeholders-are-fragile-for-programmatic-editing.md) — the fragility problem this ADR resolves.
[2] [ADR 0029](0029-the-status-vocabulary-is-per-doc-type-sourced-from-one-doctypespec-field-not-one-global-list-validated-against-every-template-s-own-dialect.md) — same "unify a scaffold contract across registered types, backed by a fitness function" pattern.
