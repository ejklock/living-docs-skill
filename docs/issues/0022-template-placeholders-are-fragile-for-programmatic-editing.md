---
type: Issue
title: Template placeholders are fragile for programmatic editing
description: Give body placeholders a fenced, uniform marker format (or a --body-file option) so programmatic edit tooling does not depend on byte-exact angle-bracket matching.
status: closed
timestamp: 2026-08-03T12:05:00Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## Template placeholders are fragile for programmatic editing

Found dogfooding living-docs as the issue tracker of a heavily-automated repo.

### Problem

Body placeholders like `<Observable, testable condition for "done".>` require byte-exact
matching to replace — angle brackets, punctuation inside quotes, and a period placed
*inside* the placeholder text all have to match exactly for an exact-string edit tool to
find them. We hit this in practice: a `.` inside a placeholder broke an automated edit that
otherwise matched the surrounding text correctly. This is a tooling-agnostic problem — any
consumer performing exact-string or regex replacement to fill a scaffold is exposed to it.

### Scope

Included: either (a) a fenced, uniform placeholder marker format (e.g. `{{ACCEPTANCE}}`)
that programmatic tooling can match on a stable token instead of prose-shaped punctuation,
or (b) a `--body-file` option on `new`/`brief` that supplies the whole body up front and
skips the placeholder scaffold entirely. Either resolves the fragility; they are not
mutually exclusive.

Explicitly out: changing the frontmatter placeholder shapes (`title`/`description` already
have a dedicated fix path via issue 0021).

### Acceptance

- Every body placeholder in every type's template uses one uniform marker syntax that does
  not embed prose punctuation inside the token being matched.
- An automated edit that fills every placeholder by matching the uniform marker succeeds
  regardless of what prose the placeholder's own hint text contains.
- If `--body-file` is added: it is byte-for-byte written as the body, with no further
  placeholder substitution expected.

### Plan

Introduce the uniform marker format in the template-rendering step (`brief`'s pre-fill and
`new`'s raw scaffold), then update the skill/docs authoring guidance that quotes the old
angle-bracket placeholders.
