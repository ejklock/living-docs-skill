---
name: okf-knowledge-format
description: Author and validate knowledge as OKF (Open Knowledge Format) bundles — a directory of markdown files with YAML frontmatter, where every concept is one .md file with a REQUIRED `type` field, reserved index.md/log.md files, bundle-relative cross-links, and a permissive conformance model. Use when standing up or maintaining a knowledge bundle/catalog, writing or normalizing a concept document's frontmatter, deciding how to structure markdown knowledge for agents to consume, or checking a corpus against OKF conformance. The canonical spec is vendored under reference/SPEC.md and refreshed from GitHub via scripts/update-spec.sh.
version: "0.7.0"
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

## Using this skill (progressive disclosure)

This SKILL.md is a **slim stub** — a trigger plus a task->topic router. The `living-docs` CLI holds the full OKF details and discloses them progressively. **Before authoring anything, load the topic for your task:**

- `living-docs skill okf-knowledge-format --list` — discover every topic.
- `living-docs skill okf-knowledge-format --topic <topic>` — load that topic.

Piped output is minified JSON (machine default); `--plain` for human text, `--json` to force JSON. Topics: model, procedure, concept, index, log, about. The vendored spec lives at reference/SPEC.md.

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
- Writing a **concept document** or normalizing its frontmatter → `living-docs skill okf-knowledge-format --topic concept`.
- Adding or regenerating a directory **`index.md`** → `living-docs skill okf-knowledge-format --topic index`; or a **`log.md`** → `living-docs skill okf-knowledge-format --topic log`.
- Deciding how to cross-link concepts, cite sources, name a `type`, or reviewing the core model / frontmatter fields / bundle structure → `living-docs skill okf-knowledge-format --topic model`.
- Checking a corpus for **OKF conformance** (the five hard rules above), authoring a concept or maintaining a directory step by step, or refreshing the vendored spec from upstream (`scripts/update-spec.sh`) → `living-docs skill okf-knowledge-format --topic procedure`.
