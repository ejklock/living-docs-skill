---
type: ADR
title: "fmt repairs a record's CLI-owned fields: it seeds a missing timestamp and owner and aligns the title heading, leaving all other prose byte-identical"
description: fmt seeds a missing timestamp from the adding commit's author date, a missing required owner from git config, and rewrites the title heading from the frontmatter title; every other body byte stays unchanged; supersedes the frontmatter-only fmt
owner: Evaldo Klock
status: Accepted
supersedes: 0047
timestamp: 2026-10-24T10:15:28Z
---

# 0071. fmt repairs a record's CLI-owned fields: it seeds a missing timestamp and owner and aligns the title heading, leaving all other prose byte-identical

This decision supersedes [ADR 0047](/adr/0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md), keeping its guarantee that `fmt` never rewrites prose and narrowing it by one line: the record's title heading.

## Context

[ADR 0047](/adr/0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md) made `fmt` frontmatter-only after a body-reflow pass deleted 12951 lines from one bundle. `fmt` now orders the frontmatter keys and leaves every byte below the closing `---` unchanged.

The failures consumers hit are elsewhere. Of 41 bundles under the owner's workspace, 25 fail `check` (`living-docs check docs --plain` per repository, October 2026); the dominant kinds are non-canonical hand-written frontmatter, an ADR without owner, and a heading that disagrees with the title. A usage audit over four harness accounts' transcripts found 516 whole-file `Write` calls into numbered records: agents rewrite the record, guess the frontmatter, and lose the timestamp the CLI wrote, which no verb restores (`set` refuses `timestamp`; `fmt` does not seed it). On a hand-written ADR with a mismatched heading, `fmt` today adds `description: ""` and leaves the missing timestamp, the missing owner and the heading as they were.

Each missing field has a deterministic source: the timestamp in the git history of the file, the owner in git config (as `new` already seeds it), the heading in the frontmatter title (as `set title` already rewrites it, [ADR 0061](/adr/0061-set-title-retitles-a-record-frontmatter-heading-filename-and-every-in-bundle-link-refused-only-on-a-terminal-or-superseded-record.md)).

## Decision

We will make `living-docs fmt` repair the CLI-owned fields of a record, by default, and keep every byte of prose unchanged.

- **Frontmatter order.** As before: the canonical key order of the type.
- **Missing `timestamp`.** Seeded from the author date of the commit that added the file; when the file was never committed, the current UTC time, as `new` stamps it. A present timestamp is never changed, and `set` still refuses the key: the timestamp is the record's creation seal.
- **Missing `owner`** on a type that requires one: seeded from git config `user.name`, then `user.email`, the rule `new` uses. When neither is set, the field stays absent and `check` keeps reporting it. A present owner is never changed.
- **Title heading.** The record's first heading keeps its level and its `NNNN.` prefix and takes the frontmatter title as its text. When the frontmatter has no title, the heading text seeds it.
- **The body is untouched otherwise.** Every byte below the closing `---` other than that one heading line stays identical. Missing sections, unfilled slots and the description are prose; the author writes them.
- **`fmt --check`** reports the records these repairs would change, writes nothing, and exits 1 when any would; a record path still selects one record.

Rejected alternatives:

- **An opt-in `fmt --repair`.** The default is what agents run; a flag they must remember goes unused, as the standalone `index` call did before [ADR 0065](/adr/0065-mutation-verbs-leave-the-bundle-indexed-new-set-and-supersede-regenerate-the-touched-type-index-in-the-same-run.md).
- **Let `set` write `timestamp`.** It turns the creation seal into an editable field and invites guessed dates.
- **Keep `fmt` as it is.** The top failure cluster in consumer bundles stays a hand repair.

## Consequences

**Easier / gained:**
- A record written by hand, or migrated from another tool, becomes conformant in its CLI-owned fields with one command, and a brownfield bundle can adopt the gate.
- The timestamp a whole-file rewrite lost is recovered from history.

**Harder / accepted trade-offs:**
- `fmt` on a brownfield bundle rewrites more records than before (heading lines); `fmt --check` shows them first.
- `fmt` now reads git history and config. Outside a git repository the timestamp falls back to the clock and the owner stays absent.
- The guarantee of [ADR 0047](/adr/0047-living-docs-fmt-is-frontmatter-only-and-the-record-body-stays-byte-identical.md) is narrowed from "the body is byte-identical" to "the body is byte-identical except the title heading".

**Follow-ups:**
- `check` advice that names `fmt` covers the heading and owner kinds.
- The procedure and check topics and `fmt --help` describe the repairs.

## Verification

**Implementation impact:** `living-docs-core/src/commands/fmt.rs`, `living-docs-core/src/record.rs` (owner seeding shared with `new`), the heading rewrite shared with `living-docs-core/src/commands/set/retitle.rs`, `cli/src/commands/fmt.rs`, `skills/living-docs/rules/procedure.md`, `skills/living-docs/rules/check.md`, `README.md`, `cli/tests/*`.

**Verification criteria:**
- A committed hand-written ADR without timestamp gets the author date of its adding commit; an uncommitted one gets a timestamp within the test's run window; a record with a timestamp keeps it byte-for-byte.
- An ADR without owner gets git config `user.name`; with no git identity configured, the owner stays absent and `check` reports it.
- A heading `# 0003. Other heading` under `title: Hand` becomes `# 0003. Hand`; every other body byte is identical, compared by hash.
- `fmt --check` on that record exits 1, lists it, and writes nothing; a second `fmt` writes nothing.
