---
type: ADR
title: Mermaid validation runs in-process via merman-core, not a Docker mermaid-cli shell-out
description: Replace the Docker `mermaid-cli` shell-out in `check`/`check --mermaid-only` with the pure-Rust `merman-core` parser, gated by a conformance corpus that fails CI if the parser ever diverges from the expected accept/reject on our fixtures.
status: Accepted
supersedes:
superseded_by:
tags: [documentation, tooling, mermaid, determinism, dependencies]
timestamp: 2026-07-18T16:10:15Z
---

# 0013. Mermaid validation runs in-process via merman-core, not a Docker mermaid-cli shell-out

## Context

`check` validates every ` ```mermaid ` fence in the corpus so a broken diagram fails the
doc-gate. Today `living-docs-core/src/check/mermaid.rs` does this by shelling out to the pinned
`minlag/mermaid-cli:11.4.2` Docker image — the real mermaid.js parser wrapped by `mmdc`. The
docblock is explicit that this is "the real parser, not a hand-rolled grammar check", and the
weight comes from `mermaid-cli` bundling puppeteer + headless Chromium to *render*.

Two frictions pull against this:

- **A runtime Docker dependency for a validation step.** `check` needs a running Docker daemon
  to validate a diagram, so a plain `cargo install`ed binary cannot fully self-check; CI must
  provide Docker; contributors without Docker get a degraded `check`. For a tool whose north star
  is a deterministic, self-contained Rust binary (ADR 0001), a Chromium-in-Docker dependency for
  *syntax validation* is disproportionate — validation only needs `parse()` (valid/invalid), never
  rendering.
- **The determinism boundary (ADR 0001).** The tool must be reproducible from its inputs and hold
  no heavyweight external runtime it does not control. Shelling out to a container image is an
  external-process boundary that the leak-gate work (ADR 0011/0012) deliberately refused for
  detection; mermaid validation is the last place that boundary is crossed.

The blocker to removing Docker had been that no pure-Rust mermaid parser existed. That is no longer
true: a `cargo search` surfaced several pure-Rust implementations, and a spike (recorded below)
measured `merman-core` against our own fixture corpus.

## Decision

We will replace the Docker `mermaid-cli` shell-out with **`merman-core`** (the parser crate of the
`merman` project, MIT OR Apache-2.0), used **parse-only**: `Engine::new().parse_diagram_sync(text,
ParseOptions::strict())`, where `Err` marks an invalid diagram and `Ok(Some(_))` a valid one. No
rendering, no layout, no Docker, no JS engine.

The decision is **bounded by a conformance corpus, not by trust in an alpha crate.** The existing
`10-mermaid-valid` / `11-mermaid-invalid` fixtures (expanded to cover each diagram type the corpus
uses) become a fitness function: a committed test asserts the parser accepts every valid fixture
and rejects every invalid one, and fails CI the moment `merman-core` diverges from that expected
accept/reject. The corpus, not the crate's version number, is the trust anchor.

Rejected alternatives:

- **Embed mermaid.js via a Rust JS engine (`rquickjs`/`deno_core`).** Gives exact parity, but
  imports a JS runtime plus a browser-globals shim for mermaid's parse path — fragile across
  mermaid upgrades, and a heavier dependency than the problem warrants once a pure-Rust parser
  reaches corpus parity.
- **Hand-roll a Rust grammar** (`pest`/`chumsky`). Dominated by adopting `merman-core`, which
  already covers 25 diagram types with a stated "1:1 parity with pinned upstream Mermaid baseline"
  design goal — reimplementing that is cost with no benefit and a larger drift surface.
- **Keep Docker, degrade gracefully.** Rejected: the objective is to remove the runtime Docker
  dependency, not to soften it.

## Consequences

**Easier / gained:**
- `check` / `check --mermaid-only` become pure in-process Rust — no Docker daemon, no Chromium, a
  single self-contained binary that fully self-checks. CI drops the Docker-for-mermaid setup.
- The determinism boundary (ADR 0001) is held everywhere: the last external-process dependency in
  the check path is gone.
- Coverage widens from "whatever the image parses" to 25 diagram types, all validated in-process
  and orders of magnitude faster than a container round-trip.

**Harder / accepted trade-offs:**
- **Crate maturity.** `merman-core` is young (0.7.x; the umbrella `merman` is 0.8.0-alpha) and
  solo-maintained. Mitigated by pinning the version and by the conformance corpus that fails CI on
  any parity regression — we depend on *measured* behavior, not on a stability promise.
- **Dependency footprint.** `merman-core` pulls ~170 transitive crates (chrono, logos, web-time,
  …). This is a one-time compile cost and a supply-chain surface; accepted in exchange for removing
  the Docker runtime dependency (the trade is compile-time deps for a lighter, self-contained
  runtime).
- **Parity is bounded by the corpus, not proven universally.** A diagram type or construct the
  corpus does not exercise could diverge from mermaid.js unnoticed. Mitigated by growing the corpus
  when new diagram shapes enter the docs.

**Follow-ups:**
- Implementation issue: add `merman-core` to `living-docs-core`, rewrite `check/mermaid.rs` to use
  the `Engine`, expand the conformance corpus, and remove the Docker path and its
  `DockerUnavailable` handling.
- If `merman-core` is abandoned or diverges materially, revisit via a superseding ADR (the rejected
  embed-JS option is the fallback of record).

## Verification

**Implementation impact:** `living-docs-core/Cargo.toml` (the `merman-core` dependency),
`living-docs-core/src/check/mermaid.rs` (Engine-based validation replacing the Docker shell-out and
its `DockerUnavailable` outcome), the CI workflow / Makefile (drop the Docker-for-mermaid
requirement), and `skills/living-docs/tests/fixtures/10-mermaid-valid` /
`11-mermaid-invalid` (expanded conformance corpus).

**Verification criteria:**
- `check` validates ` ```mermaid ` fences with no Docker daemon present; a broken diagram still
  fails the gate and a valid one passes. — fitness function (integration test, no Docker).
- The conformance corpus test asserts `merman-core` accepts every `10-mermaid-valid` diagram and
  rejects every `11-mermaid-invalid` diagram; it fails on any parity divergence. — fitness function.
- `living-docs check docs` stays green over `docs/` with this ADR indexed.

# References

[1] [merman — parity-focused headless Rust Mermaid parser](https://github.com/Latias94/merman). Available at: https://crates.io/crates/merman-core. Accessed on: 2026-07-18.
