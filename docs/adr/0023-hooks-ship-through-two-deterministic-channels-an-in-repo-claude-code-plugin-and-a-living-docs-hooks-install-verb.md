---
type: ADR
title: "Hooks ship through two deterministic channels: an in-repo Claude Code plugin and a living-docs hooks install verb"
description: Distribute the ADR 0021 enforcement layers to consumer projects through two deterministic channels — an in-repo Claude Code plugin whose hooks resolve via CLAUDE_PLUGIN_ROOT, and a `living-docs hooks install` verb that materializes the same scripts from the embedded corpus for every other harness — leaving install.sh a skill-stub copier.
owner: Evaldo Klock
status: Deprecated
tags: [cli, distribution, enforcement, hooks, plugin]
timestamp: 2026-07-29T12:43:28Z
---

> **DEPRECATED — do not act on this record.** It has no successor. Run `living-docs read records` for what is in force.

# 0023. Hooks ship through two deterministic channels: an in-repo Claude Code plugin and a living-docs hooks install verb

## Context

[ADR 0021](/adr/0021-enforcement-layers-ship-with-the-repo-write-gate-hook-session-teaching-and-pre-commit-doc-gate.md)
made *this* repository self-enforcing: `block-docs-handwrite.sh` and `session-context.sh`
live in `skills/living-docs/hooks/`, `.claude/settings.json` wires them, and
`.githooks/pre-commit` runs the doc-gate. It closed the gap for contributors to the bundle
and explicitly deferred the gap for everyone else — its own follow-up asks for
"a `living-docs hooks install` verb so consumer projects wire the same layers through the
CLI instead of copying files by hand".

That gap is now measured. `install.sh` — the only supported way to adopt the bundle — copies
`SKILL.md` stubs (plus `okf-knowledge-format`'s `reference/`) into a harness skills directory
and generates the Cursor and Copilot pointer files. It never copies `hooks/`, and it never
touches any harness settings file. A consumer project therefore adopts the *instructions*
and none of the *gates*, which inverts the ADR 0019 constraint the whole enforcement design
rests on: instructions never block; only gates block.

Copying the scripts is not sufficient on its own. The wiring committed here is
`"$CLAUDE_PROJECT_DIR"/skills/living-docs/hooks/block-docs-handwrite.sh` — a path that
resolves only inside a checkout of this bundle. Any distribution has to relocate the scripts
*and* rewrite the command that points at them, which is exactly the class of mechanical,
single-correct-output work that [ADR 0001](/adr/0001-living-docs-cli.md) assigns to the CLI
rather than to a shell installer merging JSON with `jq`.

Two properties of the existing implementation bound the solution and keep it small. The
`rust-embed` corpus in `cli/src/skill.rs` is declared `#[folder = "../skills/"]` with no
include/exclude filter, so `skills/living-docs/hooks/*.sh` are **already** compiled into the
binary — materializing them needs no new embedding. And the write-gate already reads
`LIVING_DOCS_BUNDLE` (default `docs`) for its ADR 0020 directory scope, so pinning the scope
in a consumer project is a matter of emitting one environment variable, not of teaching the
shell script to discover anything.

The residual force is harness asymmetry. Claude Code has a first-class plugin format that
bundles skills, hooks, and commands together and resolves paths through
`${CLAUDE_PLUGIN_ROOT}`; the block rules already exist there as the external
`living-docs-enforcer` plugin in the `ai-configs` repository — the duplication ADR 0021
accepted as a trade-off. OpenCode, Codex, Cursor and Copilot have no equivalent, and Cursor
and Copilot have no write-time hook surface at all.

## Decision

We will distribute the enforcement layers through **two channels, both deterministic and
both owned by this repository**, and leave `install.sh` a skill-stub copier.

**1. A Claude Code plugin, vendored in this repo, with the repo root as its source.**
`.claude-plugin/plugin.json` and `.claude-plugin/marketplace.json` declare a `living-docs`
plugin whose marketplace entry is `"source": "./"`. The repo root is already the plugin
layout Claude Code expects — `skills/<name>/SKILL.md` is auto-discovered, so the existing
`skills/` tree needs no duplication — and it costs no extra copy, because the marketplace
repository is cloned regardless and `"./"` reuses that clone. `hooks/hooks.json` at the repo
root (auto-discovered; no `plugin.json` key needed) registers `block-docs-handwrite.sh` on
`PreToolUse` (`Write|Edit|MultiEdit`) and `session-context.sh` on `SessionStart`, addressing
both as `"${CLAUDE_PLUGIN_ROOT}"/skills/living-docs/hooks/<script>` so they resolve from the
plugin cache. Adoption is `/plugin marketplace add ejklock/living-docs-skill` then
`/plugin install living-docs@living-docs` (`--scope project` to commit the choice). This
**absorbs the external `living-docs-enforcer`**: the block rules get one home, retiring the
"block rules exist in two places" trade-off ADR 0021 accepted.

**2. A `living-docs hooks install` verb, for every other harness.** It materializes the hook
scripts from the embedded corpus into `<project>/.living-docs/hooks/` (mode `0755`), emits
the harness wiring, and installs the pre-commit doc-gate. It is idempotent — re-running
converges on the same bytes — and supports `--dry-run`. Removal is the sibling subcommand
`living-docs hooks uninstall`, not a flag on `install`. It never merges
JSON by hand-rolled string surgery: an existing settings file is parsed, the living-docs hook
entries are replaced by identity, and unrelated entries are preserved verbatim.

**3. The write-gate scope is resolved at install time, not at hook time.** The verb resolves
the project's docs bundle with the same resolver the CLI already uses for `--docs-dir`, and
pins the result into the generated wiring as `LIVING_DOCS_BUNDLE`. The shell script keeps its
existing env-var contract and gains no discovery logic. When no bundle resolves, the verb
fails with the flag to pass rather than silently defaulting to `docs`.

**4. `install.sh` grows no hook logic.** It keeps copying skill stubs and generating pointer
files, and gains one line naming the correct channel per harness. Rationale: JSON merging in
`bash` is fragile, and duplicating the wiring in a shell installer would put the mechanics
outside the deterministic layer that owns them.

## Consequences

**Easier / gained:**
- A consumer project gets write-time enforcement, not just instructions — one command on
  Claude Code, one CLI verb elsewhere.
- The `${CLAUDE_PLUGIN_ROOT}` indirection removes the broken-path failure mode: no consumer
  needs a checkout of this bundle for the hooks to resolve.
- The block rules collapse to a single home, closing the duplication ADR 0021 accepted.
- Hook distribution becomes testable in this repo's CI like any other CLI verb, instead of
  being verified only by a human following README prose.

**Harder / accepted trade-offs:**
- Two channels mean two adoption paths to document and keep in sync. Accepted: they share
  one source of truth — the scripts in `skills/living-docs/hooks/` — and differ only in
  how the wiring addresses them.
- Cursor and Copilot still get no write-time gate; they retain the pre-commit and CI gates
  only. Accepted: fail-open by design, unchanged from ADR 0021.
- Plugin hooks and `.claude/settings.json` hooks fire independently, so a contributor to
  *this* repo who also installs the plugin runs both — a duplicated SessionStart notice and
  a doubled (still correct) block. Accepted rather than removing this repo's committed
  wiring, which is what guarantees enforcement for contributors who install nothing.
- Materialized scripts in a consumer project can drift from the binary that wrote them.
  Mitigated by idempotent re-install, not by a runtime version check.
- Retiring the external `living-docs-enforcer` requires a deprecation pass in the `ai-configs`
  repository, which is outside this repo's CI.

**Follow-ups:**
- Deprecate `living-docs-enforcer` in `ai-configs` and point its README at the plugin here.
- Consider a `living-docs hooks check` verb reporting drift between materialized scripts and
  the embedded corpus.

## Verification

**Implementation impact:** `.claude-plugin/plugin.json`, `.claude-plugin/marketplace.json`,
`hooks/hooks.json`, `cli/src/main.rs` (subcommand + dispatch), a new `hooks` command module,
`cli/tests/hooks_install.rs`, `install.sh` (pointer line), `README.md`, `CONTRIBUTING.md`,
`skills/living-docs/templates/claude-hard-rules.md` (rule 10).

**Verification criteria:**
- `living-docs hooks install --dir <tmp>` writes both hook scripts mode `0755` under
  `<tmp>/.living-docs/hooks/`, and their bytes equal the embedded corpus entries.
- Running the verb twice leaves the target tree byte-identical to the first run
  (idempotence), and `--dry-run` writes nothing while reporting the same plan.
- Against a settings file carrying an unrelated hook entry, the verb preserves that entry
  verbatim and the resulting file parses as JSON.
- The generated wiring pins `LIVING_DOCS_BUNDLE` to the resolved bundle; with no resolvable
  bundle the verb exits non-zero naming the flag to pass.
- `living-docs hooks uninstall` removes the living-docs entries and leaves unrelated entries
  intact.
- Every `command` in `hooks/hooks.json` is prefixed by `${CLAUDE_PLUGIN_ROOT}`, and every
  path it names exists in the plugin bundle.
- Fitness function: `cli/tests/hooks_install.rs` covers the criteria above and a manifest
  test asserts `.claude-plugin/*.json` parse and reference only existing files; both run in
  `make check`.
