---
type: Issue
title: set accepts title so a live record can be retitled without hand-editing frontmatter
description: "A record that is still Proposed has no CLI path to correct its title: set covers only status, description and owner, and the rule forbids hand-editing frontmatter, so the only options are supersede or delete and recreate."
owner: Evaldo Klock
status: closed
timestamp: 2026-09-21T16:55:12Z
---

## 0047. set accepts title so a live record can be retitled without hand-editing frontmatter

While reviewing a fresh bundle, a `Proposed` ADR was born with a wrong count in its title (three hooks; the decision settled on two). `living-docs set` accepts `status`, `description` and `owner`, and the CLI-first rule forbids hand-editing frontmatter, so the record had to be deleted and recreated through `new`, which also rewrote its filename and timestamp. The procedure topic already names the remedy: a deterministic frontmatter mutation done by hand becomes a `set` key.

### Scope

- `set <ref> title <value>` on any record type: rewrites the frontmatter `title`, the numbered heading line, and renames the file to the new slug while keeping the number.
- Every in-bundle reference to the old filename is repointed in the same transaction, so no link breaks; references outside the bundle are named, never edited.
- Refused only on a record whose status is terminal for its type, or `Superseded`, with the pointer to `supersede`.
- `index` regenerates the row on the next run; `check` reports a heading that no longer matches the title as an advisory.

### Decision

Renaming the file is part of the verb, chosen over keeping the old slug because the slug is derived from the title and a mismatch is what `check` would otherwise have to tolerate.

The gate moved from the seed status (`Proposed`/`Draft`/`open`) to the terminal statuses while the work was scoped, and [ADR 0061](../adr/0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md) carries the reasoning: the seed-status gate is zero-risk but leaves `supersede` as the only remedy for a typo in an accepted record — the exact cost this issue set out to remove. Repointing the in-bundle links is what makes the wider gate safe.

### Acceptance

- `set adr/0011 title "..."` on a live record rewrites frontmatter, heading and filename, and `check` passes after `index`.
- The same call on a terminal or `Superseded` record exits non-zero and names `supersede`.
- A title that produces an existing slug is refused.
- Every in-bundle record citing the old filename points at the new one.
