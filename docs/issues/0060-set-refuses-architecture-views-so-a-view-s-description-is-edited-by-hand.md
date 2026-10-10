---
type: Issue
title: set refuses architecture views, so a view's description is edited by hand
description: set resolves only adr, prd, issue and research, leaving the named-identity types without the verb the hard rules require.
status: open
timestamp: 2026-10-10T19:16:42Z
---

## 0060. set refuses architecture views, so a view's description is edited by hand

Found adopting 0.21.0 in a downstream bundle (relent). `new view` creates an architecture view with the placeholder description `<One sentence — what this view shows.>`, and the hard-rules template says the author owns `description`. But `set` rejects the type.

Reproduction: `living-docs new view "Rings" --kind component`, then `living-docs set view/rings description "x"` exits 2 with "'view' is not a valid type; expected one of adr, prd, issue, research". `set architecture/rings description x` fails the same way. The constitution, the other named-identity record, is refused too. The result is a hand frontmatter edit for every view, against the CLI-first rule.

A view carries no status or owner, so only `description` (and `title`, with its reference rewrite) applies.

### Scope

Let `set` address records with named identity (view by slug, constitution by name) for `description` and `title`. Kept: `status` and `owner` rejected on these types, since the registry gives them neither.

### Acceptance

- `living-docs set view/<slug> description "..."` writes the description and `check` stays OK.
- `set view/<slug> title "..."` renames the file and rewrites in-bundle references, as it does for numbered records.
- `set view/<slug> status Accepted` exits 2 and says views carry no status.
- The constitution accepts `description`; the type-list error message names the new types.
- `living-docs check docs` stays OK.
