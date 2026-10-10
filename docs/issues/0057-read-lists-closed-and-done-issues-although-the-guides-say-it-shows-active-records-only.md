---
type: Issue
title: read lists closed and done issues although the guides say it shows active records only
description: read withholds only Superseded and Deprecated records, so closed and done issues, which the registry marks terminal, fill the agent entry point.
status: open
timestamp: 2026-10-10T19:16:42Z
---

## 0057. read lists closed and done issues although the guides say it shows active records only

Found adopting 0.21.0 in a downstream bundle (relent): of 366 records, `living-docs read` printed 220 closed issues, 56% of the index output (about 13,600 of 24,500 tokens), and they also swell `read --topic` and `--contract` results.

Cause: `is_in_force` in `living-docs-core/src/commands/effective.rs` matches only `superseded` and `deprecated`. It ignores `DocTypeSpec::terminal_statuses` (issue: `closed`, `done`), which `is_retired` already uses for `set title` and for the authoring advisories ([ADR 0063](/adr/0063-authoring-advisories-skip-retired-records.md)).

Reproduction: `living-docs new issue "Alpha"`, `living-docs set issue/0001 status closed`, `living-docs read`. The closed issue is listed and `withheld` is 0.

Contradicts `skills/living-docs/SKILL.md` line 59 and `skills/living-docs/rules/semantic-index.md` line 30: "active records only (superseded/deprecated withheld)".

### Scope

`read` withholds every record `DocTypeSpec::is_retired` reports, so closed and done issues leave the default view and count in `withheld`. Kept: supersede-chain collapsing, `--topic`, `--contract` and `--full`. A flag to include retired records (for example `--all`) is the escape hatch; the name is chosen in the change.

### Acceptance

- `read` on a bundle with one open and one closed issue lists only the open one and reports `withheld: 1`.
- An issue with status `done` is withheld the same way.
- `read` with the include-retired flag lists the closed issue.
- The `read` wording in SKILL.md and semantic-index.md matches: closed and done issues are withheld.
- `living-docs check docs` stays OK.
