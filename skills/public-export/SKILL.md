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
metadata:
  type: skill
  layer: procedural
  tags: [publishing, open-source, privacy, leak-prevention, export, on-demand]
allowed-tools: Read, Grep, Glob, Bash, AskUserQuestion
---

# public-export — ship the method, keep the moat

Your written method was never the moat — the practiced judgment, the accumulated
lessons, and the decision trail are. So you *can* publish the scaffold and a curated
showcase of your reasoning (a strong credibility signal) while keeping the
accumulation private. This skill is the disciplined way to do that without leaking.

**Mental model: the public repo is a build artifact of the private one.** You never
hand-edit it; you *generate* it. One-way flow: private (source of truth) → build →
gate → clean-history publish. In this repo the deterministic parts are the
`living-docs` tool itself — `export --visibility` (the allowlist build) and
`leak-gate` (the fail-closed backstop), decided in [ADR 0010](/adr/0010-public-export-is-a-deterministic-allowlist-build-with-a-leak-gate-publish-is-a-human-gated-procedure.md).
This skill owns the **judgment** the tool cannot: what belongs in the curated
showcase, and the human review of the diff before any push.

The split is deliberate (ADR 0001 + ADR 0010): the tool is deterministic and never
runs destructive git; the irreversible clean-history publish is a human step this
skill documents, never a tool subcommand.

## The three buckets (decide per doc, explicitly)

| Bucket | Examples | Public? |
|---|---|---|
| **Scaffold** — the method | `skills/*/SKILL.md`, templates, engine source, README, LICENSE | yes |
| **Accumulation** — the moat | full lessons, memory DB, client specs, in-flight bets, outcome data | **never** |
| **Curated showcase** — portfolio | hand-picked, cleaned ADRs/research (e.g. a "why I rejected X" record) | yes, deliberately |

The showcase is the only nuance: each showcased doc is elevated **one at a time** by
setting `visibility: showcase` on it, never by publishing `docs/adr/**` wholesale.
Curation is manual on purpose.

**Derived artifacts sit in the same bucket as their sources.** Embeddings, traces,
and eval outputs derived from Accumulation content are Accumulation — an embedding is
a **copy** of its source text, not an opaque transform. A memory DB (vectors
included) never exports, and that line is the reason, so the rule survives future
judgment calls.

## How private stays separate from public (the tool never judges at publish)

Do **not** trust an LLM to decide, at export time, what is private — that is
LLM-as-judge on a security boundary, the exact failure this system forbids (ADR
0009). The judgment is made **once, by the human, at authoring time**, recorded as
**data** in the living-docs/OKF frontmatter, and enforced deterministically forever:

- Every OKF concept doc carries **`visibility: private | public | showcase`** in
  frontmatter, set when the doc is written and confirmed by you.
- **Missing or unknown `visibility` ⇒ private** (default-deny). Omission can never
  publish something by accident. `living-docs check` validates the domain (a typo
  fails), so the rule is an instrument, not a vibe.
- The export is driven by that declaration: `living-docs export --visibility
  public,showcase <out>` materializes only allowlisted docs, so **no LLM judgment
  sits in the publish path**.
- The **leak gate is the backstop**: even a mismarked-public doc is caught if it
  links to a withheld doc, or carries a secret.

The AI's only role is at *authoring*: when it writes an ADR/PRD/BDR/research note, it
proposes a `visibility` (defaulting to private) for you to confirm — never to decide
what ships at publish.

## Hard rules

- **Allowlist, never denylist.** Only `visibility: public|showcase` docs export
  (default-deny). A denylist leaks the day you add a new private doc and forget it.
- **Visibility is declared at authoring time, never judged at publish.** Missing ⇒
  private.
- **Never hand-edit the public repo.** It is regenerated. Hand edits become a second
  source that silently diverges.
- **The gate vetoes the push.** `living-docs leak-gate` must pass before publishing.
  It fails closed on a private doc present in the bundle, a published doc linking to a
  withheld doc, or a secret/PII pattern match. Fix every finding at the *source*
  (the doc's `visibility`, the link, or the secret), never by editing the built output.
- **Strip the narrative, keep the credit.** Remove internal decision narrative when
  curating a showcase doc; **keep external provenance/attribution**. Stripping
  research ≠ stripping credit.
- **A leaked secret is compromised — rotate it.** Force-push/orphan history removes a
  secret from `main` but not from forks/clones/caches/the SHA. History hygiene is the
  backstop; the gate + allowlist are the real control.
- **git push is human-gated.** The tool builds and gates; it never pushes. You review
  the diff, then push by hand.
- **The destination repo is created PRIVATE, always.** Create with `gh repo create
  <owner>/<name> --private`, push the built+gated bundle, verify once more while it is
  still private, and only then flip it public (`gh repo edit <owner>/<name>
  --visibility public`). Never create a public repo and push in one shot — the first
  commit is instantly world-readable and a leak cannot be recalled.

## Procedure

1. **Set visibility at the source.** Confirm each doc that should ship carries
   `visibility: public` (scaffold) or `visibility: showcase` (curated portfolio);
   everything else stays absent/private. `living-docs check docs` must be green (it
   validates the visibility domain).
2. **Build the allowlist bundle.** `living-docs export --visibility public,showcase
   <out>` materializes only the allowlisted docs into `<out>` — default-deny, so a
   private or absent-visibility doc never lands there.
3. **Gate.** `living-docs leak-gate <out>`. It exits non-zero on any private doc in
   the bundle, any published doc linking to a withheld doc, or any secret/PII pattern
   match. Fix every finding at the source (visibility, link, or secret), rebuild, and
   re-gate — never edit `<out>` by hand.
4. **Create the destination repo PRIVATE (first time only).** `gh repo create
   <owner>/<name> --private --description "<one-liner>"`.
5. **Clean-history publish (with the human).** In the still-private destination,
   create the curated release commit (orphan branch or history filter that keeps only
   `<out>`), review the diff, then `git push --force-with-lease`. Private keeps the
   real granular history; public shows curated release commits.
6. **Flip to public only after verifying clean while private.** Re-run `living-docs
   leak-gate` against the pushed tree, eyeball it, then `gh repo edit <owner>/<name>
   --visibility public`. Anything ever wrong is caught while still private.
7. **Record** the decision (what is published, what stays private) via `living-docs`.

## Distinct from / composes

- **`living-docs`** — owns document authoring, the `visibility` field (ADR 0009), and
  records the publish decision; this skill does not author docs.
- **`okf-knowledge-format`** — the export respects OKF; this skill removes *internal*
  narrative from showcased docs, not the format.

## Provenance — instrumentalization, not invention

The "public repo as generated artifact / one-way source→public" pattern is standard
docs-as-code practice; allowlist-over-denylist is the least-privilege/default-deny
security principle applied to publishing; orphan-branch history hygiene and "rotate a
leaked secret" are established git-secrets guidance. This skill is the composition and
the calibrated leak gate, not a new method.
