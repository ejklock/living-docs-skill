# Domain Records

A **domain record** holds what a project's domain means: its terms and its business rules. It is a living record in `docs/context/` with `type: Context`, updated in place like an architecture view; git history is its trail. It has no number and no status.

## Where a term or a rule lives

| It is | Its home |
|---|---|
| A term or a rule the domain keeps, stated as it holds now | a domain record |
| A choice expensive to reverse, with rejected alternatives | an ADR (`read guide adr`) |
| Work to do, or a cheap-to-reverse choice made while doing it | an issue |

Materiality decides, as for every record: a domain record is earned by a rule a future reader would pay to rediscover. Do not write one per module. A term or rule born in an issue's Decision starts there and is promoted (see below).

## Layout

- One record per bounded context. Group records in folders by `--dir`, one folder per bounded context, to any depth; the author chooses the taxonomy, the CLI only validates the segments.
- `living-docs write new domain "<Title>" [--dir <path>]` scaffolds `docs/context/<path>/<slug>.md`. Each `--dir` segment must be a slug; an absolute path, `..` or an empty segment is refused before any write. Without `--dir` the record sits at the root of `docs/context/`.
- `living-docs manage index domain` generates `docs/context/index.md`, listing every record by folder with its title and description. Never hand-edit it; `check` reports a hand-kept index that differs from its regeneration.

## Shape

- `## Terms`: one term per line, defined as the domain uses it. A term lives in one file only; every other file links to it.
- `## Rules`: one `###` heading per rule. The heading is the rule's name and its link anchor, so name it for what it states and keep it stable.
- A decision rule with more than two conditions is a table (conditions as columns, outcome last), not prose.
- A decision table is proved by `check` only when the line right before it is `<!-- decision-table -->` or `<!-- decision-table: first -->`, under a `###` rule of a domain record; an unmarked table is never read. Every column but the last is a condition whose header lists its values, `name (v1, v2, ...)`; a cell is one value, a comma-separated set, or `-` for any; the last column is the outcome, compared as trimmed text. By default every combination must match a row and no combination may match two rows with different outcomes; with `first`, every combination must match a row and no row may be fully shadowed by earlier rows. `check --advisories` lists each `decision-table` finding: a combination no row covers, a contradiction, a shadowed row, a value outside its declared domain, a malformed marker or header, or a table over 4096 combinations, too large to prove. The check reports what the table leaves missing or contradictory; it never judges which outcome is right.
- A lifecycle is linked to a state view (`living-docs write new view "<Title>" --kind state`, `read guide architecture-diagrams`), never redrawn in the record.
- Delete a section the record has no content for.

## Linking a rule

Link a rule by the heading's anchor. A link is relative to the linking file: from an issue or an ADR (one level below `docs/`) it is `../context/<path>.md#<heading-slug>`; from a record nested in `docs/context/`, count the folders to the target, for example `../rules.md#late-fee` from `docs/context/billing/fees/fees.md` to `docs/context/billing/rules.md`. The slug is the heading lowercased, punctuation dropped, spaces as hyphens, with a `-1`, `-2` suffix for a repeated heading. `living-docs check` resolves it; plain `check` prints only a count, and `check --advisories` lists each `anchor-missing` link, so a renamed or removed rule heading shows every link it broke. Schedule each advisory; fix the link or restore the heading.

## Promotion at issue close

A rule born in an issue's Decision moves to the domain record when the issue closes: add it there as a `###` rule under `## Rules`, then replace the rule's text in the issue with a link to it. The issue keeps the reason for the choice; the record holds the rule.

## The glossary

The glossary stays a record in `docs/context/`: the crosscutting terms and acronyms no single bounded context owns. It follows the same shape and the same rule: a term is defined in one file only.

## Retired: the hand-kept context index

The `context-index` template and its hand-kept `docs/context/index.md` are retired. The generated index replaces them: run `living-docs manage index domain`. For splitting a large doc by meaning, see `read guide semantic-index`.
