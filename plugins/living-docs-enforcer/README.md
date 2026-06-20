# living-docs-enforcer

An **opt-in** Claude Code plugin that enforces the [Living Docs](../../skills/living-docs/SKILL.md) *mechanical* invariants at the **git-commit boundary**. It bundles the `living-docs` skill and adds a `PreToolUse` hook that runs the skill's `lint-docs.sh` whenever Claude is about to run `git commit`.

> **Status: SCAFFOLD.** This is a correct, runnable skeleton. The enforcement *default* (block vs. warn) is still an open maintainer decision — see [The block-vs-warn knob](#the-block-vs-warn-knob).

This plugin is **recommended for teams that want the docs floor enforced, not just advised.** Solo / exploratory work can skip it; the skill alone (without this plugin) still provides the templates and the linter to run by hand.

## What it does

On every `git commit` the hook runs `lint-docs.sh` against your docs bundle (`./docs` by default) and decides:

| Situation | lint-docs result | Decision |
|---|---|---|
| Docs that **exist** are malformed / orphan / broken-link / supersede-broken | exit 1 | **BLOCK** by default (configurable → warn) |
| Docs are clean | exit 0 | allow, silent |
| A structural code change touches **no** `docs/` file (the *absence proxy*) | n/a | **warn only — never blocks** |
| lint-docs / git missing, no docs bundle, usage error | exit 2 / absent | allow with a warning (never wedges your commit) |

## Install (opt-in)

This plugin lives inside the `living-docs-skill` repo at `plugins/living-docs-enforcer/`. Add the repo as a marketplace and install the plugin:

```bash
# add this repo as a plugin marketplace (points at plugins/living-docs-enforcer/.claude-plugin/marketplace.json)
/plugin marketplace add ejklock/living-docs-skill

# install the enforcer
/plugin install living-docs-enforcer
```

Installing the plugin installs **both** the bundled `living-docs` skill and the commit-boundary hook. The skill is bundled by reference (a symlink to the repo's single `skills/living-docs/` source) so there is exactly one copy of `lint-docs.sh` — the hook calls that copy via `${CLAUDE_PLUGIN_ROOT}/skills/living-docs/scripts/lint-docs.sh`.

## The block-vs-warn knob

The one knob, read from the environment by the hook:

```bash
export LIVING_DOCS_ENFORCE=block   # default — sound violations block the commit
export LIVING_DOCS_ENFORCE=warn    # sound violations print to stderr and ALLOW the commit
```

(Optional) point the linter at a non-standard bundle root:

```bash
export LIVING_DOCS_BUNDLE=documentation   # default: docs
```

> **⚠ OPEN DECISION (pending the maintainer).** Whether the shipped default for the *sound* checks should be `block` or `warn` is **not yet settled.** The scaffold ships `block` (fail-safe: a docs floor that does not block is easy to ignore), but a team adopting this mid-stream may prefer `warn` for a grace period. This is deliberately a configurable knob, not a hard-wired policy — flip it per-repo / per-CI until the maintainer fixes the default.

## What this enforces vs. what it can't

**It forces the verifiable floor** — the half of Living Docs that has a *sound oracle* (the doc is in the tree and demonstrably wrong), exactly what `lint-docs.sh` checks:

- every concept doc has OKF frontmatter with a non-empty `type`
- `index.md` / `log.md` carry no frontmatter (except the bundle-root `okf_version`)
- every concept file is listed in its directory's `index.md` (no orphans)
- every directory index is reachable from the bundle-root index
- every local markdown link resolves
- a `status: Superseded` record carries a resolvable `superseded_by`

**It warns on the absence proxy** — a structural code change that touches no `docs/` file gets a non-blocking reminder (invariant 5: "no structural change without its doc"). This **never blocks**: there is no sound oracle for *"an ADR belongs here"*, and a hard block would just be satisfied with an empty placeholder doc (Goodhart). The reminder is a nudge for a human, not a gate.

**It does NOT — and cannot — force a *meaningful* ADR in flow order.** Whether a decision was *worth* recording, whether the ADR's reasoning is sound, whether docs match the code's actual behavior (invariant 1, docs-first) or each fact has exactly one home (invariant 2, semantic half) — none of these have a mechanical oracle. They stay with the **reviewer / human**, exactly as `lint-docs.sh` itself documents ("invariants 1 and 2's semantic half are NOT mechanical — they stay with the reviewing agent"). This plugin is the *floor*, not the ceiling.

## Trust / proof

`lint-docs.sh` is exercised by a fixture corpus (`tests/lint-docs/`) that asserts it stays silent on clean bundles and catches each mechanical violation it claims to — one violation per dirty fixture. That parity corpus is what makes the linter trustworthy enough to gate a commit on. (The corpus lands with the linter-hardening change; the hook wraps the same CLI either way.)

## Files

```
plugins/living-docs-enforcer/
├── .claude-plugin/
│   ├── plugin.json          # manifest: name, components (skills + hooks)
│   └── marketplace.json     # marketplace entry (source: ./)
├── hooks/
│   ├── hooks.json           # PreToolUse / matcher: Bash → block-docs-drift.sh
│   └── block-docs-drift.sh  # the commit-boundary decision logic
├── skills/
│   └── living-docs -> ../../../skills/living-docs   # bundled by symlink (single source)
└── README.md
```
