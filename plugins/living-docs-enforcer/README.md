# living-docs-enforcer

An **opt-in** Claude Code plugin that enforces the [Living Docs](skills/living-docs/SKILL.md) *mechanical* invariants at the **git-commit boundary**, **diff-aware**. It bundles a self-contained copy of the `living-docs` skill and adds a `PreToolUse` hook that runs the skill's `lint-docs.sh` in `--ratchet` mode whenever Claude is about to run `git commit` — so it blocks **only the violations a commit introduces** and grandfathers any pre-existing legacy debt.

This plugin is **recommended for teams that want the docs floor enforced, not just advised.** Solo / exploratory work can skip it; the skill alone (without this plugin) still provides the templates and the linter to run by hand.

## What it does

On every `git commit` the hook runs `lint-docs.sh --ratchet HEAD` against your docs bundle (`./docs` by default) and decides:

| Situation | ratchet result | Decision |
|---|---|---|
| The commit **introduces NEW** malformed / orphan / broken-link / supersede-broken docs | exit 1 | **BLOCK** (default; downgrade → warn) |
| Docs are clean, or only **pre-existing** debt is present | exit 0 | allow, silent (legacy debt grandfathered) |
| A structural code change touches **no** `docs/` file (the *absence proxy*) | n/a | **warn only — never blocks** |
| lint-docs / git missing, no docs bundle, usage / baseline error | exit 2 / absent | allow with a warning (never wedges your commit) |

Because the ratchet never punishes you for debt you didn't touch, blocking by default is **adoption-safe**: you can drop the plugin into a repo with pre-existing docs debt and only *new* drift will block.

## Install (opt-in)

This plugin lives inside the `living-docs-skill` repo at `plugins/living-docs-enforcer/`. Add the repo as a marketplace and install the plugin:

```bash
# add this repo as a plugin marketplace (points at plugins/living-docs-enforcer/.claude-plugin/marketplace.json)
/plugin marketplace add ejklock/living-docs-skill

# install the enforcer
/plugin install living-docs-enforcer
```

Installing the plugin installs **both** the bundled `living-docs` skill and the commit-boundary hook.

The skill is **vendored as a real, self-contained copy** inside the plugin (`skills/living-docs/`, not a symlink), so the plugin installs correctly from **any** source — a full-repo clone, a zip download, or a copy of just the `plugins/living-docs-enforcer/` directory. The hook calls that vendored copy via `${CLAUDE_PLUGIN_ROOT}/skills/living-docs/scripts/lint-docs.sh`.

### Keeping the vendored copy in sync (for repo maintainers)

The canonical source of the skill is the repo's single `skills/living-docs/` tree. The vendored copy under the plugin is a real directory, so it could drift — neutralized by an instrument (a constraint without an instrument is a vibe):

```bash
# edit the skill at the canonical source, then regenerate the vendored copy:
make sync-plugin-skill

# assert they are identical (this runs in CI and as part of `make check`):
make check-plugin-skill-sync
```

**Always edit the skill at the source (`skills/living-docs/`), then run `make sync-plugin-skill`.** Never hand-edit the vendored copy under `plugins/`. `make check-plugin-skill-sync` exits non-zero if the two trees differ, and it is wired into `.github/workflows/ci.yml` and `make check`, so a drifted copy fails the build.

## The enforcement knob

The default is **`block`** — this is the **settled, decided policy**: a docs floor that does not block is easy to ignore, and the diff-aware ratchet makes block-by-default safe (only *new* drift blocks). The one documented opt-out, read from the environment by the hook:

```bash
export LIVING_DOCS_ENFORCE=block   # default (decided) — NEW sound violations block the commit
export LIVING_DOCS_ENFORCE=warn    # NEW sound violations print to stderr and ALLOW the commit
```

(Optional) point the linter at a non-standard bundle root:

```bash
export LIVING_DOCS_BUNDLE=documentation   # default: docs
```

## What this enforces vs. what it can't

**It forces the verifiable floor** — the half of Living Docs that has a *sound oracle* (the doc is in the tree and demonstrably wrong), exactly what `lint-docs.sh` checks, scoped diff-aware to what the commit introduces:

- every concept doc has OKF frontmatter with a non-empty `type`
- `index.md` / `log.md` carry no frontmatter (except the bundle-root `okf_version`)
- every concept file is listed in its directory's `index.md` (no orphans)
- every directory index is reachable from the bundle-root index
- every local markdown link resolves
- a `status: Superseded` record carries a resolvable `superseded_by`

**It warns on the absence proxy** — a structural code change that touches no `docs/` file gets a non-blocking reminder (invariant 5: "no structural change without its doc"). This **never blocks**: there is no sound oracle for *"an ADR belongs here"*, and a hard block would just be satisfied with an empty placeholder doc (Goodhart). The reminder is a nudge for a human, not a gate.

**It does NOT — and cannot — force a *meaningful* ADR in flow order.** Whether a decision was *worth* recording, whether the ADR's reasoning is sound, whether docs match the code's actual behavior (invariant 1, docs-first) or each fact has exactly one home (invariant 2, semantic half) — none of these have a mechanical oracle. They stay with the **reviewer / human**, exactly as `lint-docs.sh` itself documents ("invariants 1 and 2's semantic half are NOT mechanical — they stay with the reviewing agent"). This plugin is the *floor*, not the ceiling.

## Relationship to the harness-agnostic enforcement (`enforcement/`)

This plugin is the **Claude-Code in-loop layer** plus the skill bundle. It is **not** the only place the floor is enforced — it calls the **same** `lint-docs.sh --ratchet` CLI as its harness-agnostic siblings in the repo's [`enforcement/`](../../enforcement/README.md) directory:

- `enforcement/pre-commit.sh` — a plain **git pre-commit hook** that enforces the same ratchet for *any* committer (human, Pi, OpenCode, Cursor, Copilot). Bypassable with `git commit --no-verify`.
- the CI **Action** — the unbypassable backstop that runs the same ratchet on every PR.

All three call **one CLI** at the same boundary. This plugin adds the Claude-Code in-loop ergonomics (the hook fires inside the agent's `git commit`, with the `block|warn` knob) and ships the skill so the templates + linter install together.

## Trust / proof

`lint-docs.sh` is exercised by a fixture corpus (`tests/lint-docs/`) that asserts it stays silent on clean bundles and catches each mechanical violation it claims to (one violation per dirty fixture), plus a diff-aware ratchet corpus (`tests/lint-docs/run-ratchet.sh`) that asserts a NEW violation blocks while pre-existing debt is grandfathered. That parity corpus is what makes the linter trustworthy enough to gate a commit on.

## Files

```
plugins/living-docs-enforcer/
├── .claude-plugin/
│   ├── plugin.json          # manifest: name, components (skills + hooks)
│   └── marketplace.json     # marketplace entry (source: ./)
├── hooks/
│   ├── hooks.json           # PreToolUse / matcher: Bash → block-docs-drift.sh
│   └── block-docs-drift.sh  # the commit-boundary decision logic (diff-aware ratchet)
├── skills/
│   └── living-docs/         # SELF-CONTAINED vendored copy (real dir, not a symlink)
│                            # regenerate from the canonical source with `make sync-plugin-skill`
└── README.md
```
