---
type: ADR
title: In-bundle links are written relative to the linking file, and check advises on a slash-rooted link
description: "The relative link style is canonical for links between records: guides and slot hints say so, the CLI writes only it, and check emits an advisory, not a violation, for an in-bundle slash-rooted link"
owner: Evaldo Klock
status: Proposed
timestamp: 2026-11-16T02:33:32Z
---

# 0084. In-bundle links are written relative to the linking file, and check advises on a slash-rooted link

## Context

The bundle holds two link styles and the tool takes no side. The guides tell authors to write `/`-prefixed bundle-relative links (`skills/living-docs/rules/okf-format.md`, the okf-knowledge-format model and procedure, the check checklist, the hard-rules template), and the `new issue` template's SUMMARY hint shows `/adr/NNNN-slug.md`. The CLI writes sibling-relative links: `manage index` rows and `write supersede` callouts carry `0001-foo.md`. `check` resolves every style and says nothing. This repository mixes both (225 `/` links and 227 relative ones in `docs/` on 2026-11-15).

The styles do not behave the same on the host most projects use. On GitHub a `/adr/0007-slug.md` link resolves against the repository root and returns 404 when the bundle sits under `docs/`; a sibling or `../` link resolves in the browser, in editors and in `check`. A downstream adopter (issue 0083) had to write its own decision to deviate from the guides.

## Decision

We will make the relative link canonical: a link between two records of the bundle is written relative to the linking file (`0001-foo.md` for a sibling, `../adr/0007-slug.md` across type directories). The guides and the `new` template slot hints say so, with the GitHub reason. `write new`, `manage index` and `write supersede` write only that style.

`check` emits an advisory, never a violation, for an in-bundle link written in the `/` style, and it keeps resolving both styles, so an existing bundle still passes. A link to a target outside the bundle stays relative, since it has no `/` form.

Rejected: the `/` style as canonical. It reads the same from every file, but it breaks on GitHub, and the CLI's own index and supersede writers would have to change in every adopter's bundle.

Rejected: accepting both styles with no advisory. It changes nothing, but the guides keep steering authors to links that 404 and mixed bundles stay mixed.

## Consequences

**Easier / gained:**
- One answer for authors, the CLI and the gate, and links that work where the bundle is browsed.
- No CLI writer changes its output.

**Harder / accepted trade-offs:**
- Every `/` link in a bundle raises an advisory until it is rewritten, this repository's own 225 included; rewriting them is follow-up work.
- The advisory has to tell an in-bundle `/` link from an external absolute URL.

## Verification

**Implementation impact:** the guides named above, the `new` templates' slot hints, `living-docs-core/src/check/` (the new advisory kind) and its tests.

**Verification criteria:**
- The guides and the slot hints name the relative style as canonical, with the GitHub reason.
- `write new`, `manage index` and `write supersede` write only relative links.
- `check` emits the advisory for an in-bundle `/` link, a test proves it fires, and a bundle with mixed styles still exits 0.
