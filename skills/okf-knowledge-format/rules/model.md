# OKF format details

## Core model (§2)

| Term | Meaning |
|---|---|
| **Knowledge Bundle** | Self-contained directory tree of knowledge docs — the unit of distribution (git repo, archive, or subdir). |
| **Concept** | One unit of knowledge = one markdown document. May describe a tangible asset (table, API) or an abstract idea (metric, process). |
| **Concept ID** | The file path within the bundle minus `.md`. `tables/users.md` → `tables/users`. |
| **Frontmatter** | YAML metadata block at the top of the file. |
| **Body** | Everything after the frontmatter. |
| **Link** | A markdown link asserting a relationship; its *kind* comes from surrounding prose, not the link. |
| **Citation** | A link to an external source backing a claim. |

---

## Frontmatter fields (§4.1)

```yaml
---
type: <Type name>                  # REQUIRED — e.g. "BigQuery Table", "API Endpoint", "Playbook"
title: <Display name>              # Recommended
description: <One-sentence summary># Recommended — feeds index/search/previews
resource: <Canonical URI>          # Recommended for real assets; omit for abstract concepts
tags: [<tag>, <tag>]               # Optional
timestamp: <ISO 8601 datetime>     # Optional — last meaningful change
# … any producer-defined keys are allowed
---
```

- `type` values are **not** centrally registered — pick descriptive, self-explanatory strings; consumers treat unknown types as generic concepts.
- Producers MAY add any extra keys; consumers SHOULD preserve unknown keys and never reject on them.

### Conventional body headings (§4.2)

There are no required body sections. Favor structural markdown (headings, tables, lists, fenced code) over prose. Use these headings when applicable: `# Schema` (asset columns/fields), `# Examples` (usage, often code blocks), `# References` (external sources, §8).

---

## Bundle structure (§3)

```
bundle/
├── index.md            # optional directory listing (progressive disclosure)
├── log.md              # optional update history
├── <concept>.md        # a concept at the root
└── <subdir>/
    ├── index.md
    └── <concept>.md
```

Directory layout is domain-independent — organize concepts however the knowledge wants. Reserved files (`index.md`, `log.md`) may appear at any level.

---

## Cross-linking (§5)

- **Absolute (bundle-relative), recommended:** start with `/`, stable across moves — `[customers](/tables/customers.md)`.
- **Relative:** standard markdown paths — `[other](./other.md)`.
- A link is an untyped directed relationship; the kind (joins-with, depends-on, references) lives in the prose. Broken links are tolerated (not-yet-written knowledge).

## References (§8)

List external sources under a trailing `# References` heading, numbered. Links MAY be absolute URLs, bundle-relative paths, or paths into a `references/` subdirectory that mirrors external material as first-class concepts.
