---
type: Issue
title: Domain records in docs/context nested by folder, and anchor-checked links
description: "Implements ADR 0099 and 0100: a domain registry type with write new domain --dir, a generated context index, nested records reached by every verb, and the anchor-missing advisory."
status: closed
timestamp: 2026-11-23T02:28:27Z
---

## 0150. Domain records in docs/context nested by folder, and anchor-checked links

Implements [ADR 0099](../adr/0099-domain-records-are-a-registry-type-in-docs-context-nested-in-folders-by-bounded-context.md) and [ADR 0100](../adr/0100-check-resolves-a-link-s-anchor-against-a-heading-of-its-target-as-an-advisory.md), the S2 step of [issue 0144](0144-engineering-docs-roadmap-domain-language-and-rules-decision-capture-issue-waves-generated-diagrams-and-a-local-html-export.md).

### Scope

- A `domain` registry row: records under `docs/context/` with `type: Context`, no number, no status vocabulary, a template with optional `## Terms` and `## Rules`.
- `write new domain "<Title>" [--dir <path>]` in the CLI and the MCP `write` tool, placing the record at any folder depth under `docs/context/`, with the path validated.
- A generated `docs/context/index.md`, grouped by folder, replacing the hand-kept one; `check`, `fmt`, `read records`, `write set` and references reach nested records.
- The `anchor-missing` advisory in `check`, with a GitHub-compatible heading slug.
- The skill rules for domain records, and the retirement of `templates/context-index.md`.

Kept: every existing type, the `architecture` views and numbered types staying one level deep, the verb surface (one new flag), and the rule that the CLI never writes a term or a rule.

### Decision




- Nested placement is a new `Identity::Nested { dir }` variant, not a flag on `Named`: every exhaustive match then decides the nested case explicitly instead of inheriting one-level behavior. Not taken: `Named { dir, nested: bool }`, which compiles through the same matches but hides the choice.
- Domain records need no per-folder `index.md`: the one generated `docs/context/index.md` links every record, so a nested folder holds only records. Not taken: an index per folder, which multiplies generated files.
- A `status:` key on an existing context file is left to its author, reported by the `status-vocabulary` advisory; `manage fmt` does not delete it. Not taken: dropping it in `fmt`, which rewrites author text.
- Retired records and anchors (owner, 2026-11-23): `anchor-missing` also judges Superseded and Deprecated records, so a historical record linking a heading renamed later keeps the advisory. Not taken: exempting retired records, as `path-missing` does.
- Links in an index row (2026-11-23, review of the context index rows): a markdown link, image, autolink or inline HTML tag in a record's description is written to the generated index as its text only, since a relative link resolves from the record's folder and not the index's; the link stays in the record. Not taken: rebasing relative links per row; forbidding links in descriptions.

### Acceptance

- `write new domain "Billing rules" --dir billing/invoices` writes `docs/context/billing/invoices/billing-rules.md`, regenerates `docs/context/index.md`, and `check` passes; without `--dir` the record lands in `docs/context/`.
- A `--dir` that is absolute, holds `..`, an empty segment, a non-slug segment, or resolves through a symlink out of the bundle exits non-zero and writes nothing.
- A bundle holding hand-written context files with `type: Context` and a hand-kept index passes `check` after `manage index`, with every file listed by folder; a second run changes no byte.
- `read records` lists domain records as in force; `write set context/billing/invoices/billing-rules.md description ...` works.
- A link `context/x.md#a-rule` whose heading does not exist raises `anchor-missing`; an existing heading, a repeated heading with its `-1` suffix and a non-ASCII heading resolve.
- Architecture views and numbered records are still found one level deep only.

### Plan


- S1: the `domain` row, `Identity::Nested`, its template and every exhaustive match, with `write new domain` placing at the root of `docs/context/` | files: living-docs-core/src/doc_type.rs, doc_type tests, skills/living-docs/templates/domain.md (create), the match sites, cli/src/args/help.rs | after: none
- S2: `--dir` on `write new` (CLI and MCP) with path validation | files: living-docs-core/src/commands/new.rs, new/fill.rs, cli/src/args/sub.rs, cli/src/commands/new.rs, mcp/src/args.rs, mcp/src/tools.rs, tests | after: S1
- S3: nested records reached by index, check, fmt, references and read; generated folder-grouped context index | files: living-docs-core/src/commands/index (named.rs, index.rs), check/placeholder.rs, check/index_drift.rs, commands/reference/lookup.rs, tests | after: S1
- S4: the `anchor-missing` advisory and the GitHub heading slug | files: living-docs-core/src/check/links.rs, a new slug module, check/kinds.rs, cli/tests | after: none
- S5: skill rules for domain records and anchors, retire templates/context-index.md, help examples | files: skills/living-docs/rules, skills/living-docs/templates/context-index.md, cli/src/args/help.rs | after: S2, S3, S4
- S6: the generated context index carries each record's description as ADR 0099 states, and `write new domain --dir` regenerates the context index like a root-level write | files: living-docs-core/src/commands/index/named.rs, living-docs-core/src/commands/index.rs, living-docs-core/src/commands/new.rs, cli/tests | after: S3
