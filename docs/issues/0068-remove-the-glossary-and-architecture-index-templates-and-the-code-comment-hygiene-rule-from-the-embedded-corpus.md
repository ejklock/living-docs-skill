---
type: Issue
title: Remove the glossary and architecture-index templates and the code-comment-hygiene rule from the embedded corpus
description: Three corpus entries no session loads leave the guide; semantic-index and context-index stay because agents read them and semantic-index points at context-index
status: open
timestamp: 2026-10-24T10:17:31Z
---

## 0068. Remove the glossary and architecture-index templates and the code-comment-hygiene rule from the embedded corpus

Found by the usage audit over four harness accounts' transcripts (October 2026): `guide` loaded `glossary`, `architecture-index` and `code-comment-hygiene` at most once each, and no transcript reads `templates/glossary.md` or `templates/architecture-index.md` directly; `rules/code-comment-hygiene.md` was read three times. Continues the corpus cut of [ADR 0057](/adr/0057-refocus-living-docs-on-a-decision-log-delete-bdr-cut-peripheral-verbs-and-check-advisories-and-consolidate-the-rule-corpus.md).

### Scope

- Delete `skills/living-docs/templates/glossary.md`, `skills/living-docs/templates/architecture-index.md` and `skills/living-docs/rules/code-comment-hygiene.md`, with every router line, topic list and test that names them.
- Before deleting each file, confirm no code path renders it (`new`, `index`, `install`); a file a code path uses stays, and the report names why.

### Decision

- `semantic-index` stays: transcripts read `rules/semantic-index.md` directly 25 times and SKILL.md routes to it. `context-index` stays: `semantic-index` builds on its template.
- Code-comment rules belong to the engineering standards, not to a docs skill.

### Acceptance

- `living-docs guide --list` no longer names the three topics, and `living-docs guide glossary` exits 2.
- `git grep -n -e 'templates/glossary' -e 'architecture-index' -e 'code-comment-hygiene' -- skills cli living-docs-core README.md` prints nothing.
- The full suite, `make check` and `se-gates check` pass, and `living-docs check docs` stays OK.

### Plan

1. Confirm no code path renders the files.
2. Delete them and the references; update the guide tests.
