---
type: ADR
title: Public export is a deterministic allowlist build with a leak gate; publish is a human-gated procedure
description: Split public-export into a deterministic Rust build (allowlist export of public/showcase docs plus a leak gate that fails on any private doc, dangling private link, or secret) and a human-gated clean-history publish that lives in a skill, never in the tool.
status: Accepted
tags: [documentation, methodology, publishing, security, visibility]
timestamp: 2026-07-17T22:36:12Z
---

# 0010. Public export is a deterministic allowlist build with a leak gate; publish is a human-gated procedure

## Context

ADR 0009 made document visibility default-deny frontmatter data — `check` validates the domain
and `index --visibility` filters a listing. That is the data foundation; it does not yet let
anyone actually ship the public part of the corpus. The missing capability is `public-export`:
turn the mixed private/public corpus into a bundle safe to publish, and publish it.

Publishing a knowledge corpus that mixes the method (safe to share) with the moat (research,
accumulated lessons, client specifics, in-flight decisions) is a **privacy boundary**, and the
failure modes are asymmetric: shipping a private doc, or a public doc that links to a private one,
or a secret embedded in otherwise-public prose, is a leak that a `git push` makes permanent. A
build that is "usually right" is not good enough — the boundary must be enforced by a
deterministic gate that fails closed.

Two forces pull in different directions:

- **Determinism boundary (ADR 0001).** The tool is deterministic by construction and holds no LLM;
  it must never *judge* which docs are public (that is the human's authoring decision, now encoded
  as visibility data) and must never run destructive git on the author's behalf. Rewriting history
  to strip private content into a clean public branch is exactly the kind of irreversible,
  judgment-adjacent operation that belongs to a human at the keyboard, not to a batch tool.
- **Mechanical enforcement.** Selecting the allowlisted docs, materializing them, and scanning the
  result for leaks is pure, reproducible computation — precisely what the tool should own so the
  author never pays tokens for it and the check never drifts from the `check`/`index` logic that
  already reads visibility.

## Decision

We will split `public-export` along the determinism boundary:

- **The tool owns the deterministic build.** Two mechanical capabilities, reusing the ADR 0009
  visibility read:
  - **Allowlist export.** `export` gains a `--visibility <csv>` filter (mirroring `index`):
    it materializes only records whose effective visibility is in the set (`public,showcase`),
    default-deny — absent visibility is private and never exported. This is the allowlist build.
  - **Leak gate.** A deterministic verification over an exported bundle that **fails closed** on
    any of three leak classes:
    1. **A private doc present** — any materialized doc whose effective visibility is not in the
       published allowlist (belt-and-suspenders over the export filter).
    2. **A dangling private link** — any bundle-relative link from a published doc whose target is
       absent from the published bundle (a public doc referencing a doc that was withheld leaks by
       reference, not just by presence).
    3. **A secret / PII hit** — a regex scan of published content for high-signal secret patterns
       (keys, tokens, credentials) and PII. This class is heuristic; it fails closed on a match but
       is understood to be advisory (false positives possible), and its pattern set is versioned.
- **The human owns the publish.** The clean-history publish (orphan branch or history filter that
  keeps only the exported bundle, then push) lives as a documented, human-gated procedure in a
  `public-export` **skill**, never as a tool subcommand. The tool produces the safe bundle and
  proves it clean; a human runs the destructive git step with the gate's green result in hand.

The export and the leak gate ship as their own vertical slices, each with tests; the skill is
authored alongside.

## Consequences

**Easier / gained:**
- A reproducible, fail-closed path from the mixed corpus to a publishable bundle — the privacy
  boundary is enforced by a gate, not by reviewer vigilance.
- The leak gate reads visibility through the same logic as `check`/`index`, so the three cannot
  drift on what "private" means.
- Destructive git stays in human hands with an explicit green gate as the precondition — no batch
  tool ever force-pushes a rewritten history on its own.

**Harder / accepted trade-offs:**
- The secret/PII regex class is heuristic: it can false-positive (blocking a safe publish until the
  pattern or the content is adjusted) and can miss a novel secret shape. It is a safety net layered
  on the visibility allowlist, not a substitute for it.
- Two new mechanical surfaces (an export flag and a leak-gate command) the tool must maintain.
- The publish procedure being a skill means it is not a single `living-docs publish` command; the
  human runs documented git steps. This is the intended cost of keeping destructive git out of the
  tool.

**Follow-ups:**
- The `public-export` skill documents the end-to-end procedure (export → leak-gate → human-gated
  clean-history publish) and is the home of the git recipe.
- A future ADR may revisit the secret/PII pattern set if it proves too noisy or too permissive.

## Verification

**Implementation impact:** `living-docs-core/src/commands/export.rs` + `cli/src/main.rs` (the
`--visibility` export filter), a new leak-gate module in `living-docs-core` + its `cli/src/main.rs`
wiring, and the `public-export` skill under `skills/`.

**Verification criteria:**
- `living-docs export --visibility public,showcase <out>` materializes only public/showcase docs;
  a private or absent-visibility doc never appears in `<out>`. — fitness function (integration
  test).
- The leak gate exits non-zero when the exported bundle contains a private doc, a public doc
  linking to a withheld doc, or a secret-pattern match; and exits zero on a clean public bundle. —
  fitness function (tests, one per leak class).
- `living-docs check docs` stays green over `docs/` with this ADR indexed.
