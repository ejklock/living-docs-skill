---
type: Issue
title: The harness matrix is six targets where four are the same copy with a different destination
description: Decide whether the six harness targets become a registry row the way doc types did, now that placement has moved from shell into Rust and the duplication is visible in one file.
status: closed
timestamp: 2026-07-30T22:34:18Z
---

<!-- OKF frontmatter above carries the tracker metadata (`status`: open | in-progress |
     closed | superseded) that previously lived only in the directory index. Everything
     BELOW the closing `---` is the issue body and MUST stay byte-identical to the
     published tracker body — strip the frontmatter when publishing. -->

## The harness matrix is six targets where four are the same copy

Follow-up from [ADR 0028](../adr/0028-the-release-binary-is-the-unit-of-distribution-install-sh-only-bootstraps-it-and-every-placement-becomes-a-cli-verb.md).

`living-docs skill install` covers six harness targets. Four of them — Claude, opencode,
Codex, and pi — are the same operation with a different destination: copy the skill
directories into a global path, or into a project-scoped path when asked. The remaining two
differ in kind: Cursor and Copilot generate a single pointer file with harness-specific
frontmatter wrapped around the same skill body.

So the matrix has two shapes and one axis of variation within each. Today that lives as
per-harness code, which is fine while it is six and is exactly how the doc-type taxonomy
looked before [ADR 0026](../adr/0026-a-single-doctype-registry-replaces-nine-hand-synced-enumerations-and-research-and-constitution-enter-as-rows.md).

The question this issue exists to answer is whether a `HarnessSpec` registry earns its
place here, or whether that would be pattern-matching on a resemblance rather than on a
cost. The honest answer is not yet known, and ADR 0028 deliberately did not guess it.

### Scope

Included: deciding, with evidence, whether the harness matrix becomes a compile-time
registry. If it does, the row carries the destination shape (global-and-project directory
pair, or single generated pointer file), the paths, and the frontmatter wrapper — the way
`DocTypeSpec` carries identity, template and index rendering.

Explicitly out: adding a harness. This issue is about the shape of the existing six, not
about growing them. If a seventh harness is requested first, that request is the evidence
and this issue should be decided before it lands.

### Acceptance

- A decision is recorded — an ADR if it changes the design, a note on this issue if the
  answer is "leave it as code and revisit at eight harnesses".
- If a registry lands: adding a harness is one row, a row that omits any field fails to
  compile, and a fitness function asserts every row's generated output actually carries
  that row's frontmatter wrapper — the shape ADR 0026 fitness function A established.

### Plan

Wait for the evidence rather than manufacture it. The trigger to decide is the first of:
a seventh harness is requested, a harness path changes upstream, or a bug is found in one
harness that is latently present in the other three.

Until one of those fires, the duplication is four short functions that are read more often
than they are edited, and a registry would be the more expensive shape.

### Outcome

Closed as decided by issue 0046: packaging is the CLI, the harness matrix leaves the repo, and `skill install --harness` is the single placement channel.
