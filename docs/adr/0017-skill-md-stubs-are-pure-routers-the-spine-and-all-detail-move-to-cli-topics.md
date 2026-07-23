---
type: ADR
title: SKILL.md stubs are pure routers; the spine and all detail move to CLI topics
description: Every embedded skill's SKILL.md becomes a pure router — frontmatter trigger, a "Using this skill" progressive-disclosure block, and a task→topic table — with all substantive prose (including the five core invariants) relocated into CLI-served topics; this supersedes ADR 0014's clause that kept the spine inline in the stub.
status: Accepted
supersedes: 0014
tags: [cli, documentation, progressive-disclosure, skill-distribution, tokens, tooling]
timestamp: 2026-07-20T15:29:03Z
---

# 0017. SKILL.md Stubs Are Pure Routers; the Spine and All Detail Move to CLI Topics

## Context

[ADR 0014](/adr/0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md)
made the CLI the server of detailed skill content (the `skills/**` tree is embedded via
`rust-embed` and served as `living-docs skill <name>` / `--topic <topic>`), and made each
harness-installed `SKILL.md` a **slim stub**. But 0014's decision point 3 deliberately kept
**substantive content inline** in the stub: the five core invariants (the "spine"), on the
resilience argument that "the stub still carries the always-true spine so the invariants hold
even without the binary on PATH."

Two things make that clause the weak seam of an otherwise-good decision:

- **The stubs are not actually pure routers.** `living-docs/SKILL.md` still carries the five
  invariants *and* a 20-line "Hard rule — the CLI owns the mechanics" block that is **triplicated**
  (it also lives in `rules/procedure.md` and `templates/claude-hard-rules.md`) — a direct
  one-home-per-fact violation. `okf-knowledge-format/SKILL.md` carries its five conformance rules
  inline; `public-export/SKILL.md` is a 142-line document with no `rules/` topics at all. Only
  `research-artifacts/SKILL.md` is already a router.
- **The resilience argument is thin.** `living-docs` is a **CLI-backed** skill: without the binary
  on PATH the agent cannot run any deterministic verb (`new`, `supersede`, `index`, `check`,
  `export`) — the entire workflow is inert. Preserving five lines of philosophy inline buys almost
  nothing when the machinery those lines govern is already unavailable. The token cost is paid on
  every load (and, for always-on Cursor/Copilot, every turn); the benefit accrues only in a
  degraded state where the skill is already non-functional.

The user's directive is a **pure casca**: one SKILL.md shell whose only job is to trigger and to
route every substantive need to a CLI topic. That is a cleaner realization of 0014's own
progressive-disclosure intent (L1 trigger → L2 router → L3 detail-on-demand), so this ADR
**supersedes 0014** rather than amending it — MADR-lite is append-only, and 0014's spine-inline
clause is load-bearing enough that editing it in place would rewrite history.

Epistemic type: **judgment** (a token-economy / resilience trade-off, no benchmark decides it).

## Decision

We will make **every embedded skill's `SKILL.md` a pure router** and relocate **all** substantive
prose — including the five core invariants — into CLI-served topic files under `rules/`.

1. **The stub carries only three things:** (a) the `name` + `description` frontmatter (the L1
   trigger, unchanged); (b) a near-the-top, imperative **`## Using this skill (progressive
   disclosure)`** block that instructs the agent to load the relevant topic via
   `living-docs skill <name> --topic <topic>` (or `--list`) **before authoring** and to operate
   from the loaded topic; (c) a **`## When to invoke`** task→topic router table. No invariants, no
   hard-rule prose, no conformance list, no procedure inline.

2. **The five core invariants (the spine) become a topic.** They relocate verbatim into
   `skills/living-docs/rules/spine.md`, served as `living-docs skill living-docs --topic spine`,
   and the router links to it. The spine is no longer duplicated in the stub.

3. **All remaining inline prose relocates to `rules/` topics, one home per fact.** The
   triplicated "CLI owns the mechanics" block is removed from `living-docs/SKILL.md` (its home is
   `rules/procedure.md`, reachable via `--topic procedure`; the project-guide copy in
   `templates/claude-hard-rules.md` is a *different artifact* — a CLAUDE.md seed — and stays).
   `okf-knowledge-format`'s conformance rules move to a `rules/` topic. `public-export`'s inline
   sections (buckets, visibility model, hard rules, procedure, composition, provenance) move to
   `rules/` topics, and its stub gains the `Using this skill` block and a `version` field it lacks.

4. **Scope is all four embedded skills** — `living-docs`, `okf-knowledge-format`,
   `research-artifacts` (already a router; audited, not rewritten), and `public-export`. The vendored
   `okf-knowledge-format/reference/SPEC.md` is **not** a topic (topics derive only from `rules/` +
   `templates/`); the stub keeps its pointer to the vendored spec.

5. **No `skill.rs` change is required.** Topic resolution is generic over `rules/`/`templates/`
   basenames, so a new `rules/spine.md` becomes `--topic spine` automatically. The other 0014
   decisions — embed the corpus, serve per topic, TTY-aware JSON/plain output, install ships only the
   stub — **carry forward unchanged**; only the spine-inline clause is reversed.

## Consequences

**Easier / gained:**
- One home per fact: the spine, the CLI-mechanics rule, and every conformance/procedure block live
  in exactly one topic file; the stub cannot drift from them because it no longer copies them.
- Lower always-on and per-invocation token cost — the stub is trigger + router only.
- A uniform shape across all four skills; `public-export` finally joins the progressive-disclosure
  model instead of being a 142-line always-inline document.

**Harder / accepted trade-offs:**
- **Without the CLI on PATH the agent no longer sees the invariants inline.** Accepted: the skill is
  CLI-backed and already non-functional without the binary, so the resilience 0014 bought was
  largely notional. The `description` frontmatter still states what the skill enforces at L1.
- **Prose updates still require a rebuild + reinstall** to reach agents via the CLI (unchanged from
  0014).
- **A one-time editorial relocation across four skills**, with the risk of losing a fact in the move —
  bounded by `cargo test -p living-docs skill`, `living-docs check docs`, and the stub↔topic audit
  below.

**Rejected alternative (the judgment critic):** *Keep the spine inline (ADR 0014 as written).* The
strongest tempting option — five lines of always-true philosophy is cheap, and it is the one thing an
agent could still honor with no binary. It loses because the binary's absence already disables every
verb the invariants describe, so the inline copy guards a state in which the skill cannot operate;
meanwhile it perpetuates a per-load token cost and a two-representation drift surface (stub spine vs a
future `rules/spine.md`). Pure routing is the cleaner equilibrium.

**Follow-ups:**
- If a stub↔topic cross-check proves worth automating, add the ADR 0014 fitness function (a test that
  every router pointer resolves to an embedded topic and every `rules/` topic is reachable from its
  stub) — currently unimplemented; this ADR does the audit by hand.

## Verification

**Implementation impact:** `skills/living-docs/SKILL.md` (drop the spine + CLI-mechanics blocks),
`skills/living-docs/rules/spine.md` (new), `skills/okf-knowledge-format/SKILL.md` +
`skills/okf-knowledge-format/rules/conformance.md` (new), `skills/research-artifacts/SKILL.md`
(audit), `skills/public-export/SKILL.md` + `skills/public-export/rules/*.md` (new topics). No Rust
change; no `install.sh` change.

**Verification criteria:**
- **Pure-router shape (fitness function, inspection):** each `SKILL.md` contains only frontmatter, a
  `## Using this skill` block, and a `## When to invoke` router — no invariant list, conformance
  list, hard-rule prose, or procedure inline. — `verify_by: inspection`
- **No lost fact:** every substantive block removed from a stub resolves to a CLI topic —
  `living-docs skill living-docs --topic spine` prints the five invariants; `--topic procedure`
  prints the CLI-mechanics rule; the okf conformance rules and every public-export section resolve to
  a `--topic`. — `verify_by: command`
- **Serving unchanged:** `cargo test -p living-docs skill` stays green (`# Living Docs` H1 and the
  `adr` topic still resolve). — `verify_by: test`
- **Doc-gate:** `living-docs check docs` stays green with this ADR indexed and 0014 superseded. —
  `verify_by: command`

# References

[1] [ADR 0014 — the CLI serves skill content from an embedded corpus; harness SKILL.md files are slim stubs (superseded by this ADR)](/adr/0014-the-cli-serves-skill-content-from-an-embedded-corpus-harness-skill-md-files-are-slim-stubs.md)
