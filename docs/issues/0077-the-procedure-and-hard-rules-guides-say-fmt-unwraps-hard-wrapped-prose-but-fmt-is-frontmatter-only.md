---
type: Issue
title: The procedure and hard-rules guides say fmt unwraps hard-wrapped prose, but fmt is frontmatter-only
description: Guides promise a behavior the binary does not have; an author who wraps prose trusts a fix that never comes.
status: closed
timestamp: 2026-10-10T19:16:42Z
---

## 0077. The procedure and hard-rules guides say fmt unwraps hard-wrapped prose, but fmt is frontmatter-only

Found adopting 0.21.0 in a downstream bundle (relent). [ADR 0047](../adr/0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md) made `fmt` frontmatter-only and superseded [ADR 0046](../adr/0046-fmt-unwraps-hard-wrapped-prose-one-paragraph-is-one-line-and-the-authoring-rule-says-so.md), but three embedded guides still say `fmt` unwraps paragraphs.

Reproduction: put a paragraph hard-wrapped at 80 columns in a record body, run `living-docs fmt docs`. Only frontmatter changes; the body stays byte-identical. Yet these lines say otherwise:

- `skills/living-docs/rules/procedure.md` lines 21-22: "`living-docs fmt` unwraps hard-wrapped paragraphs (ADR 0046)".
- `skills/living-docs/templates/claude-hard-rules.md` line 102: "(`living-docs fmt` unwraps it)". This template is copied into downstream projects' CLAUDE.md.
- `skills/research-artifacts/rules/rules.md` line 11: "`living-docs fmt` unwraps it (living-docs ADR 0046)".

### Scope

Rewrite the three passages so they state the rule the binary enforces: one paragraph is one line, written that way by the author, and `fmt` never reflows prose; it repairs only CLI-owned fields (frontmatter, and the title heading) as [ADR 0071](../adr/0071-fmt-repairs-a-record-s-cli-owned-fields-it-seeds-a-missing-timestamp-and-owner-and-aligns-the-title-heading-leaving-all-other-prose-byte-identical.md) states, which supersedes ADR 0047. Cite ADR 0071 where a citation stays, never 0046 or 0047. Kept: the one-line-per-paragraph authoring rule, and the `fmt` behavior.

### Acceptance

- `grep -rn "fmt.*unwrap\|unwraps" skills` returns no hit that claims `fmt` rewrites body prose.
- The three passages say the author keeps each paragraph on one line and `fmt` never reflows prose (it repairs only CLI-owned fields).
- No embedded guide cites ADR 0046 or 0047 as live behavior.
- `living-docs check docs` stays OK.
