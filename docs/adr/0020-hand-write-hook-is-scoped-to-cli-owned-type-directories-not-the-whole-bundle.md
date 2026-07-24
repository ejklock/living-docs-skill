---
type: ADR
title: Hand-write hook is scoped to CLI-owned type directories, not the whole bundle
description: Narrow the ADR 0019 write-time hand-write hook from the whole docs bundle to the four CLI-owned type directories (adr/bdr/prd/issues), so hand-authored types the CLI never scaffolds (research) and the bundle-root index.md are outside its scope.
status: Accepted
supersedes: 0019
tags: [check, cli, enforcement, frontmatter, hooks]
timestamp: 2026-07-24T01:34:45Z
---

# 0020. Hand-write hook is scoped to CLI-owned type directories, not the whole bundle

## Context

ADR 0019 added a PreToolUse write-time hook (`block-docs-handwrite.sh` in the
`living-docs-enforcer` plugin) that blocks three hand-write escapes — creating a
new `NNNN-*.md` record, writing an `index.md` directly, and editing a CLI-owned
frontmatter key. Its Decision scoped these blocks to *"inside a docs bundle"*: the
hook's bundle guard matched any path under `$LIVING_DOCS_BUNDLE/` (default `docs/`).

That scope is wrong, and dogfooding surfaced it. `living-docs new` scaffolds only
four types — `adr`, `bdr`, `prd`, `issue` — and `living-docs index` regenerates only
those four type indexes. `paths::dir_for` in `living-docs-core` is the single source
of truth for that set. But the docs bundle also holds **hand-authored** doc types the
CLI never scaffolds — most concretely `research` (`type: Research`, living under
`docs/research/` as `NNNN-*.md` files, see [/research/0002-ontology-tooling-codegraph-docs-bridge.md](/research/0002-ontology-tooling-codegraph-docs-bridge.md))
— plus the bundle-root `docs/index.md`, which is hand-maintained because `index` only
emits per-type indexes.

Against those, the bundle-wide hook mis-fires:

- Creating `docs/research/0003-*.md` triggers the new-record block, naming
  `living-docs new` — a verb that **cannot** scaffold research, since it is not a CLI
  type. The block is unactionable.
- Writing `docs/research/index.md` or the bundle-root `docs/index.md` triggers the
  `index.md` block, naming `living-docs index` — which regenerates neither.
- Editing a CLI-owned key (`type`/`status`/`timestamp`) inside a research record would
  trigger the frontmatter block, though research frontmatter is hand-authored.

The root cause is a category error in ADR 0019: it equated "inside the bundle" with
"CLI-owned". The CLI owns *type directories it scaffolds*, not the whole bundle.

## Decision

We will scope the hand-write hook to the **CLI-owned type directories only** —
`adr`, `bdr`, `prd`, `issues` under the bundle — mirroring `paths::dir_for`. The hook's
bundle guard becomes a directory allowlist (`CLI_OWNED_DIRS_RE`); a path that is not
directly inside one of those four directories falls through to allow (exit 0) before any
of the three block rules is evaluated. Hand-authored types (`research`) and the
bundle-root `docs/index.md` are thereby outside the hook's scope entirely — the CLI has
no `new`/`index` verb to defer them to, so blocking them can never be actionable.

The gate is by **directory**, not by frontmatter `type`, deliberately: on a create the
file has no trustworthy frontmatter yet, and the directory is the deterministic signal
of CLI ownership. This supersedes ADR 0019's *"inside a docs bundle"* scope; every other
facet of ADR 0019 (the three block rules, the `LIVING_DOCS_ENFORCE=block|warn` knob, the
fail-open contract, the canonical round-trip `check`, and point-of-use teaching) is
unchanged and remains in force.

## Consequences

**Easier / gained:**

- Hand-authored doc types (`research` today, any future non-CLI type) and the bundle-root
  `docs/index.md` are authored freely — no unactionable block naming a verb that cannot help.
- The hook's scope now tracks a single source of truth (`paths::dir_for`): the four CLI-owned
  type directories. Adding a future CLI type is a one-place change on both sides.

**Harder / accepted trade-offs:**

- The hook no longer offers *any* write-time guard for hand-authored types. Accepted:
  there is no CLI mechanic to enforce for them, so there was never a correct block to
  give — only a wrong one. The mechanical `living-docs check` invariants (frontmatter,
  indexing, links, supersede) still cover every type in every harness.
- "CLI-owned" is now encoded as a directory-name allowlist in the hook, duplicating the
  `paths::dir_for` set in shell. Accepted as a small, testable constant; the regression
  suite pins both sides.

**Follow-ups:**

- None. The enforcer plugin cache ships the change with the next version bump.

## Verification

**Implementation impact:** `plugins/living-docs-enforcer/hooks/block-docs-handwrite.sh`
(the `CLI_OWNED_DIRS_RE` constant and the single bundle-guard line) and
`plugins/living-docs-enforcer/tests/test-block-docs-handwrite.sh` (the `ac-s5-7`
section) in the `ai-configs` repository.

**Verification criteria:**

- A `Write` creating a new `NNNN-*.md` under `docs/research/`, a `Write` to
  `docs/research/index.md`, a `Write` to the bundle-root `docs/index.md`, and an `Edit`
  to a CLI-owned key inside a `docs/research/NNNN-*.md` record each exit 0 (allow).
- The pre-existing `adr/bdr/prd/issues` blocks are unchanged: a new record under
  `docs/adr/` still exits 2 naming `living-docs new`; `docs/adr/index.md` still exits 2
  naming `living-docs index`; a CLI-owned key edit under `docs/adr/` still exits 2.
- Fitness function: `tests/test-block-docs-handwrite.sh` (sections `ac-s5-1`..`ac-s5-7`)
  passes and `shellcheck` is clean on both files.
