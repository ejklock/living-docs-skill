---
name: public-export
description: >-
  Publish a clean public version of a private living-docs project — the
  method/scaffold and a curated showcase — WITHOUT exposing your research,
  decisions, accumulated lessons, client specifics, or secrets. Use when you want
  to open-source a skill, a tool, or a framework whose private repo holds the "why"
  and the moat. Treats the public repo as a generated build artifact (never
  hand-edited): a default-deny allowlist export driven by document visibility, a
  deterministic leak gate, and a human-gated clean-history publish. The
  three-bucket model — scaffold (public), accumulation (never), curated showcase
  (deliberately published). Invocable as /public-export.
version: "0.7.0"
metadata:
  type: skill
  layer: procedural
  tags: [publishing, open-source, privacy, leak-prevention, export, on-demand]
allowed-tools: Read, Grep, Glob, Bash, AskUserQuestion
---

# public-export — ship the method, keep the moat

Publish the scaffold and a curated showcase of your reasoning while keeping the accumulation
private. The public repo is a **build artifact** of the private one — generated, gated, and
human-published, never hand-edited.

---

## Using this skill (progressive disclosure)

This SKILL.md is a **pure router** (ADR 0017) — a trigger plus a task→topic router. The
`living-docs` CLI holds the full model, rules, and procedure and discloses them progressively.
**Before exporting or publishing, load the topic for your step:**

- `living-docs skill public-export --list` — discover every topic.
- `living-docs skill public-export --topic <topic>` — load that topic.

Piped output is minified JSON (machine default); `--plain` for human text, `--json` to force JSON.
Topics: about, buckets, visibility-model, rules, procedure.

---

## When to invoke

- Understanding **why** this skill exists, its build-artifact mental model, how it composes with
  `living-docs`/`okf-knowledge-format`, and its provenance → `living-docs skill public-export --topic about`.
- Deciding **what goes where** — scaffold vs accumulation vs curated showcase, and the
  derived-artifacts rule → `living-docs skill public-export --topic buckets`.
- Understanding **how private stays private** — the `visibility` frontmatter field, default-deny,
  and why the tool never judges at publish → `living-docs skill public-export --topic visibility-model`.
- Enforcing the **hard rules** (allowlist-not-denylist, gate vetoes the push, PRIVATE-first repo,
  rotate a leaked secret) → `living-docs skill public-export --topic rules`.
- Running the **export → gate → clean-history publish** steps → `living-docs skill public-export --topic procedure`.
