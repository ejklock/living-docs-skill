---
type: Issue
title: The guides prescribe /-bundle-relative links, but index and supersede write sibling-relative links and check accepts every style
description: An adopter cannot tell which link style is canonical, and the prescribed one 404s when the bundle is browsed on GitHub.
status: open
timestamp: 2026-10-10T19:17:22Z
---

## 0061. The guides prescribe /-bundle-relative links, but index and supersede write sibling-relative links and check accepts every style

Found adopting 0.21.0 in a downstream bundle (relent), which then needed an ADR to settle the question. The guides tell authors to write `/`-prefixed bundle-relative links, the CLI writes another style, and the gate enforces neither.

- Guides prescribing `/`: `skills/living-docs/rules/okf-format.md` line 19 ("Cross-link with `/`-prefixed bundle-relative paths"), `skills/okf-knowledge-format/rules/model.md` line 58 ("recommended"), `skills/okf-knowledge-format/rules/procedure.md` line 8, the check checklist and the hard-rules template. The `new issue` template's SUMMARY slot hint also shows `/adr/NNNN-slug.md`.
- What the CLI writes: `index` rows and `supersede` callouts use sibling-relative links (`0001-foo.md`, as in `docs/issues/index.md` here).
- What `check` does: resolves every style and says nothing about the choice.

Cost: a `/adr/0007-slug.md` link resolves against the repository root when a bundle is browsed on GitHub and returns 404, while sibling and `../` links resolve. A bundle that follows the guides loses working links on the host most projects use, and an adopter who notices must write a decision record to deviate, rewriting thousands of links. Hand-written `/` links and CLI-written sibling links also coexist in one bundle, as in this repository's own docs.

Reproduction: `living-docs new adr "A"`, `living-docs new adr "B"`, `living-docs supersede 0001 0002`, `living-docs index adr`. The callout and the index rows carry sibling links, against the guides; `check` passes.

### Scope

Pick one canonical style, make the CLI write it, make the guides say it, and have `check` advise on the other. The argument for sibling/relative as canonical is the GitHub 404 above: it works in the browser, in editors and in `check`, where `/` works only where the bundle root is the site root. Kept: `check` resolving both styles, so an existing bundle still passes; the advisory is not a violation. Links to targets outside the bundle stay relative, since they have no `/` form.

### Acceptance

- The guides (okf-format, okf-knowledge-format model and procedure, check checklist, hard-rules template) and the `new` template slot hints name one canonical style, with the GitHub rationale.
- `new`, `index` and `supersede` write only that style.
- `check` emits an advisory, not a violation, for an in-bundle link in the other style, and a test proves it fires.
- A bundle with mixed styles still passes `check` (exit 0).
- `living-docs check docs` stays OK.
