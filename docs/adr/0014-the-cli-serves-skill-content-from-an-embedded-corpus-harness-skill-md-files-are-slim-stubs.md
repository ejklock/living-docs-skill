---
type: ADR
title: The CLI serves skill content from an embedded corpus; harness SKILL.md files are slim stubs
description: Embed the skills/** tree in the living-docs binary and expose it via a `skill` subcommand (list, full body, per-topic); install ships a slim stub per harness that points at the CLI for detail, instead of copying the full rules/ + templates/ tree into every harness.
status: Superseded
supersedes:
superseded_by: 0017
tags: [documentation, tooling, skill-distribution, tokens, cli]
timestamp: 2026-07-18T19:40:42Z
---

# 0014. The CLI serves skill content from an embedded corpus; harness SKILL.md files are slim stubs

## Context

The three skills (`living-docs`, `okf-knowledge-format`, `research-artifacts`) are distributed to
harnesses by `install.sh` in two shapes:

- **Native SKILL.md harnesses** (Claude, OpenCode, Codex, Pi): `cp -R skills/<name>` copies the
  **whole tree** — `SKILL.md` plus `rules/` (11 files for `living-docs`), `templates/` (9), and
  `tests/` — into each harness's skills directory.
- **Cursor / Copilot**: the `SKILL.md` body is concatenated into a single always-on context file
  (`.cursor/rules/*.mdc`, `.github/instructions/*.instructions.md`).

Two frictions pull against this:

- **Token weight per harness.** Every native harness carries the full `rules/` + `templates/`
  corpus on disk, and a fat `SKILL.md` is paid for on every invocation (and, for Cursor/Copilot,
  on *every* turn, since those are always-on). Most of that detail — an ADR convention, a template
  — is only needed at the moment the agent authors that one doc type, not on every load.
- **The CLI is already the delivery vehicle.** `install.sh cli` puts a self-contained binary on
  PATH. That binary is the deterministic layer of Living Docs (ADR 0001). The detailed,
  fill-in-the-template knowledge the skill carries is exactly the kind of deterministic,
  reproducible-from-inputs content the CLI is built to serve — there is no reason it must live as
  loose files in six harness directories when the binary can serve it on demand.

The blocker had been that the CLI had no way to carry or serve that content. It can: a Rust binary
can embed an arbitrary file tree at compile time.

## Decision

We will make the **CLI the server of detailed skill content**, and make each **harness-installed
`SKILL.md` a slim stub**.

1. **Embed the corpus.** The `skills/**` tree (each skill's `SKILL.md`, `rules/`, `templates/`) is
   embedded in the `living-docs` binary at compile time (`rust-embed`, from the `cli` crate — serving
   skill docs is a front concern, so it stays out of `living-docs-core`, which holds domain + ports
   only). The binary is self-contained: no runtime file lookup, version-locked to the release that
   built it.

2. **Serve it per topic via a `skill` subcommand.**
   - `living-docs skill --list` — list embedded skills and, per skill, the available topics.
   - `living-docs skill <name>` — print the full canonical `SKILL.md` body for `<name>`.
   - `living-docs skill <name> --topic <topic>` — print one topic's detail (a `rules/<topic>.md`
     and/or `templates/<topic>.md`), so an agent pulls just the ADR conventions when authoring an
     ADR, not the whole corpus. `<topic>` maps to the `rules/`/`templates/` basenames.

   **Output format is context-aware — machine-friendly by default when piped.** Because the primary
   consumer is an agent, not a human at a terminal, the `skill` command detects whether stdout is a
   TTY (`std::io::IsTerminal`): piped/redirected (non-TTY) output defaults to **minified JSON**
   (single line, `serde_json`), and interactive (TTY) output defaults to **human-readable** markdown.
   Two explicit flags always override the auto-detection so scripts and tests are deterministic:
   `--json` forces JSON, `--plain` forces human text (mutually exclusive). This TTY-sensitivity is a
   *presentation* choice only; it does not touch the doc-authoring determinism ADR 0001 governs (the
   tool still never chooses an epistemic type or writes rationale prose), and the explicit flags make
   any invocation reproducible regardless of environment.

3. **The canonical `SKILL.md` becomes the slim stub.** It keeps only what a harness needs to
   (a) **trigger** the skill — the `name` + `description` frontmatter, unchanged — and (b) **route
   to detail** — the five invariants (the spine), the when-to-invoke table repointed to
   `living-docs skill … --topic <x>`, and a one-line "run the CLI for full detail" pointer. The
   heavy prose that today lives inline in `SKILL.md` (Procedure, Enforcement modes, brownfield
   adoption, Composition) **relocates into topic files under `rules/`** — one home per fact: the
   topic file is the home, the stub points at it.

   **The stub is self-bootstrapping (progressive disclosure).** This mirrors the standard
   Agent-Skills three-level progressive-disclosure model: L1 = the always-in-context `name` +
   `description` (the trigger); L2 = the SKILL.md body, now the slim spine + router loaded on
   invoke; L3 = the per-task detail, pulled on demand. The stub does not merely *mention* the CLI —
   it **instructs** the agent, imperatively and near the top (a `## Using this skill` block), to
   load the relevant topic via `living-docs skill <name> --topic <topic>` (or `--list` to discover)
   **before authoring**, and to operate from the loaded topic rather than the thin stub alone. The
   CLI call is the operating procedure that bridges L2 → L3, not an optional footnote.

4. **Install ships only the slim stub.** `install.sh` stops copying `rules/` + `templates/` into
   native harnesses; it installs the slim `SKILL.md` and nothing else. Cursor/Copilot already
   receive only the `SKILL.md` body, which is now the slim version automatically. The full corpus
   travels inside the binary, reachable via `living-docs skill`.

The determinism boundary (ADR 0001) is preserved: serving embedded markdown is a pure,
reproducible-from-inputs read — no LLM, no external process.

Rejected alternatives:

- **Read the corpus from an on-disk dir at runtime** (`~/.living-docs/skills/`). Lets prose update
  without a rebuild, but needs a separate install step for that content and it can drift from the
  binary version. Rejected in favor of embedding: releases already rebuild the binary, so
  version-locking the content to it is a feature, not a cost.
- **Hybrid (embedded fallback + on-disk override).** Two resolution paths to maintain for a benefit
  (hot-editing prose) nobody needs in normal use. Rejected as premature surface.
- **Serve only the whole `SKILL.md` body, no per-topic.** Simpler CLI, but the agent re-pulls the
  full body every time — it saves the always-on cost but not the per-need cost. Per-topic is the
  whole point: pay for the ADR conventions only when authoring an ADR.
- **Keep a fat `SKILL.md` and generate the slim stub mechanically at install time.** Requires a
  reliable "which lines are the spine" heuristic and leaves two representations to drift. Rejected:
  make the canonical `SKILL.md` *be* the slim stub, with detail relocated to topic files.

## Consequences

**Easier / gained:**
- Each harness carries a small stub instead of the full `rules/` + `templates/` corpus — lower
  always-on context (Cursor/Copilot) and lower per-invocation cost (native harnesses).
- One delivery vehicle: `install.sh cli` ships the binary, and the binary carries the full skill
  knowledge. No six-directory corpus to keep in sync on disk.
- Detail is pulled by need: `living-docs skill living-docs --topic adr` is a deterministic,
  scoped fetch at authoring time.

**Harder / accepted trade-offs:**
- **Prose updates require a rebuild + reinstall of the binary** to reach agents via the CLI. Accepted:
  releases already recompile the binary, and the version-lock keeps stub and corpus coherent.
- **The stub must stay a faithful index of the topics.** A `rules/` topic with no pointer from the
  slim `SKILL.md`, or a pointer to a missing topic, is drift. Mitigated by a fitness function that
  cross-checks stub pointers against embedded topics (see Verification).
- **A harness agent without the CLI on PATH loses the detail.** Accepted: `living-docs` is a
  CLI-backed skill; the CLI is a documented prerequisite (README install), and the stub still
  carries the always-true spine so the invariants hold even without the binary.
- **Editorial relocation of `SKILL.md` prose into topic files** is a one-time reorganization across
  three skills; the risk is losing a fact in the move, bounded by `living-docs check` staying green
  and the stub↔topic cross-check.

**Follow-ups:**
- Implementation slices: (S1) `skill` subcommand + `rust-embed` of `skills/**` + `--list`/body/
  `--topic`; (S2) relocate `living-docs` `SKILL.md` heavy prose into `rules/` topics and slim the
  stub, repoint the when-to-invoke table at the CLI; (S3) apply the slim-stub reshape to
  `okf-knowledge-format` and `research-artifacts`; (S4) `install.sh` stops copying `rules/` +
  `templates/`, ships only the slim stub, and the README/CONTRIBUTING document the `skill` command.
- If per-topic granularity proves too coarse or too fine in practice, revisit the topic taxonomy via
  a superseding ADR.

## Verification

**Implementation impact:** `cli/src/main.rs` (the `skill` subcommand), `cli` crate (an embed
module over `skills/**` via `rust-embed` and the topic-resolution logic — a front concern, not
`living-docs-core`), `skills/*/SKILL.md` (slimmed
to stubs), `skills/living-docs/rules/*.md` (new topic files receiving relocated prose), `install.sh`
(stop copying `rules/`+`templates/`; ship the slim stub only), and `README.md` / `CONTRIBUTING.md`
(document `living-docs skill`).

**Verification criteria:**
- `living-docs skill --list` lists all three skills and their topics; `living-docs skill living-docs`
  prints the slim body; `living-docs skill living-docs --topic adr` prints the ADR conventions +
  template. An unknown skill or topic exits non-zero with a usable error. — fitness function
  (integration test).
- Output format follows the TTY: piped (non-TTY) invocations emit minified JSON by default,
  interactive (TTY) invocations emit human-readable markdown; `--json` and `--plain` override the
  auto-detection and are mutually exclusive. Errors stay plain text on stderr with a non-zero exit in
  every mode. — fitness function (integration test pins the mode via the explicit flags).
- Every topic referenced by a slim `SKILL.md` when-to-invoke pointer resolves to an embedded topic,
  and every embedded `rules/` topic is reachable from its stub — a test fails on either gap. —
  fitness function (stub↔topic cross-check).
- `install.sh` (dry-run) for a native harness installs the slim `SKILL.md` and does **not** copy
  `rules/` or `templates/`. — fitness function (installer dry-run assertion).
- Each slim `SKILL.md` carries a near-the-top, imperative `## Using this skill` block that
  instructs the agent to load a topic via `living-docs skill <name> --topic <topic>` (and `--list`
  to discover) before authoring — progressive disclosure L2 → L3 is an instruction, not a mention. —
  fitness function (inspection / stub-shape check).
- `living-docs check docs` stays green over `docs/` with this ADR indexed.

# References

[1] [rust-embed — embed files into a Rust binary at compile time](https://crates.io/crates/rust-embed). Available at: https://crates.io/crates/rust-embed. Accessed on: 2026-07-18.
