---
type: Issue
title: new has no --description flag, yet description is CLI-owned frontmatter
description: Add a --description flag (and a later describe verb) so new no longer seeds a placeholder the user must hand-edit, contradicting the CLI-ownership contract.
status: closed
timestamp: 2026-08-03T12:05:00Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## new has no --description flag, yet description is CLI-owned frontmatter

Found dogfooding living-docs as the issue tracker of a heavily-automated repo.

### Problem

The docs-authoring contract says frontmatter is CLI-owned and hand-writes are blocked (the
`block-docs-handwrite.sh` PreToolUse hook), but `living-docs new <type> "<title>"` seeds
`description: <One sentence — the change and its motivation.>` — a placeholder the user
*must* hand-edit to get a real description, because no other verb sets it. `description` is
one of the few frontmatter keys the hook leaves editable by design, which avoids a hard
block, but the contract's own framing ("frontmatter is CLI-owned") still does not hold for
this field in practice.

### Scope

Included: a `--description "<sentence>"` flag on `living-docs new` so the field is set at
creation time like `title` already is. A follow-up `living-docs describe <NNNN> "<sentence>"`
for editing the description on an existing record, keeping frontmatter fully CLI-mediated
end to end.

Explicitly out: making `description` a hard-blocked frontmatter key in the hook — it should
stay editable as a fallback, the same way `tags` does.

### Acceptance

- `living-docs new <type> "<title>" --description "<sentence>"` writes the given sentence
  into frontmatter instead of the placeholder.
- Omitting `--description` keeps today's placeholder behavior (no regression).
- A `describe` verb (or equivalent) can update `description` on an existing record without
  touching any other frontmatter key.

### Plan

Add the flag to `new`'s argument parser and template-fill step; add a thin `describe`
subcommand that reuses the same frontmatter-write path `status` already uses for a single
key. No ADR needed — this is additive and doesn't change validated behavior.
