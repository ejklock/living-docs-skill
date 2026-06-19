---
name: okf-knowledge-format
description: Author and validate knowledge as OKF (Open Knowledge Format) bundles — a directory of markdown files with YAML frontmatter, where every concept is one .md file with a REQUIRED `type` field, reserved index.md/log.md files, bundle-relative cross-links, and a permissive conformance model. Use when standing up or maintaining a knowledge bundle/catalog, writing or normalizing a concept document's frontmatter, deciding how to structure markdown knowledge for agents to consume, or checking a corpus against OKF conformance. The canonical spec is vendored under reference/SPEC.md and refreshed from GitHub via scripts/update-spec.sh.
metadata:
  type: skill
  layer: procedural
  tags: [documentation, knowledge-format, okf, frontmatter, metadata, conformance]
---

# OKF — Open Knowledge Format

Represent knowledge as an **OKF bundle**: a directory tree of UTF-8 markdown files with YAML frontmatter, designed to be read by humans, written by agents, and exchanged across organizations with no required tooling. "If you can `cat` a file, you can read OKF." This skill is the repo's standard for *how knowledge markdown is structured* — frontmatter shape, reserved files, cross-links, and conformance.

The authoritative rules live in the vendored spec at `reference/SPEC.md` (OKF v0.1). This SKILL.md is the operational summary; when a detail is ambiguous, open `reference/SPEC.md` and follow it.

> **Provenance — not ours.** OKF is a published, vendor-neutral standard from **Google Cloud Platform** (OKF v0.1, 2026-06-12), not a format coined in this repo. We adopt and vendor it (`reference/SPEC.md` is refreshed from upstream via `scripts/update-spec.sh`). Source: GOOGLE CLOUD PLATFORM, *Open Knowledge Format — Specification v0.1* — full citation in `../../references/prior-art-landscape.md`.

---

## Hard rules (these define conformance — §9)

1. **Every non-reserved `.md` file has a parseable YAML frontmatter block** delimited by `---` on its own line at the top and a closing `---`.
2. **Every frontmatter block has a non-empty `type` field.** `type` is the only required field. Everything else is optional.
3. **Reserved filenames are reserved.** `index.md` (directory listing, §6) and `log.md` (update history, §7) must follow their defined structure and must **not** be used for concept documents.
4. **`index.md` carries no frontmatter** — the sole exception is the bundle-root `index.md`, which MAY declare `okf_version: "0.1"` (§11).
5. **Consume permissively.** Never reject a bundle for missing optional fields, unknown `type` values, unknown extra keys, broken cross-links, or a missing `index.md`. OKF stays useful as bundles grow and get partially agent-generated.

---

## When to invoke

- Standing up a new knowledge bundle/catalog, or organizing existing markdown knowledge into one.
- Writing a **concept document** or normalizing its frontmatter → start from `templates/concept.md`.
- Adding or regenerating a directory **`index.md`** → `templates/index.md`; or a **`log.md`** → `templates/log.md`.
- Deciding how to cross-link concepts, cite sources, or name a `type`.
- Checking a corpus for **OKF conformance** (the four hard rules above).
- Refreshing the vendored spec from upstream → `scripts/update-spec.sh` (see "Keeping the spec current").

---

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

---

## Procedure

### Author a concept
1. Copy `templates/concept.md`. Set a non-empty, descriptive `type` (hard rule 2).
2. Fill recommended fields (`title`, `description`, and `resource` if it maps to a real asset). Add `tags`/`timestamp` if useful.
3. Write the body in structural markdown; use `# Schema`/`# Examples` where they fit. Cross-link with bundle-relative `/…` paths.
4. Add a `# References` section for any externally-sourced claim.

### Maintain a directory
1. Keep `index.md` listing the directory's concepts, descriptions mirroring each concept's frontmatter `description` (`templates/index.md`).
2. If the scope tracks history, append a dated entry to `log.md` (newest first, ISO 8601 dates — `templates/log.md`).
3. Declare `okf_version: "0.1"` in the bundle-root `index.md` frontmatter only.

### Check conformance
Walk the four hard rules: every non-reserved `.md` has parseable frontmatter; every block has non-empty `type`; reserved files follow §6/§7; root `index.md` is the only `index.md` with frontmatter.

---

## Keeping the spec current

The spec is **vendored** (verbatim) at `reference/SPEC.md` with provenance in `reference/SPEC.source.md`. To pull the latest from GitHub:

```bash
skills/okf-knowledge-format/scripts/update-spec.sh          # default ref: main
skills/okf-knowledge-format/scripts/update-spec.sh v0.2     # a tag/branch/commit
```

The script overwrites `reference/SPEC.md`, rewrites `reference/SPEC.source.md` (URL, ref, retrieval time, sha256), and reports whether the content changed. **If it changed, review the diff and reconcile the Hard rules / field tables above** before committing — this SKILL.md must not drift from the vendored spec. The vendored copy is the offline source of truth; the script is the only sanctioned way to update it.

---

## Notes

- `type` is the single point of required structure. When unsure whether something belongs in frontmatter or body: identifying/routing metadata → frontmatter; explanation and evidence → body.
- OKF references domain schemas (Avro, Protobuf, OpenAPI) rather than replacing them — link out, don't inline a competing schema.
- Relationship to **living-docs**: that skill governs a repo's *internal* doc system (ADR/PRD/BDR/constitution) and its no-drift discipline; OKF is the portable, exchange-oriented *format* for knowledge bundles. Use OKF when the knowledge is a catalog meant to be consumed by agents or shared across orgs; use living-docs conventions for in-repo decision/requirement records. They compose: a living-docs `docs/context/` or research corpus can be authored as an OKF bundle so its frontmatter and indexing are spec-conformant.
