---
type: ADR
title: Secret and PII detection stays deterministic — a curated ruleset plus Shannon entropy, never ML
description: The leak gate's secret/PII scan deepens with a curated gitleaks-style regex ruleset, Shannon-entropy detection for generic high-entropy secrets, and deterministic PII (email, phone, CPF, Luhn-checked card), and explicitly rejects ML/NER PII detectors and shelling out to external scanners because both break the determinism boundary.
status: Accepted
tags: [leak-prevention, methodology, privacy, publishing, security]
timestamp: 2026-07-17T23:20:00Z
---

# 0011. Secret and PII detection stays deterministic — a curated ruleset plus Shannon entropy, never ML

## Context

ADR 0010 gave the leak gate a third leak class — a secret/PII scan — with a deliberately
minimal starter pattern set (PEM key, AWS AKIA, a secret-assignment shape, email), and noted
the set is heuristic, versioned, and expected to be revisited. This is that revisit.

The question raised: shouldn't the gate use a *more advanced* PII/secret engine rather than a
handful of hand-rolled regexes? Surveying the ground:

- **Best-in-class secret scanners** (gitleaks, trufflehog) are **Go CLIs, not embeddable Rust
  crates**, and under the hood they are exactly *curated regex rulesets plus Shannon-entropy
  detection* — not a fundamentally different technique. There is no dominant, maintained Rust
  library to embed for this.
- **Best-in-class PII detection** (Microsoft Presidio and peers) is **NER/ML** — it recognizes
  entities with models. That is **non-deterministic** and pulls heavy ML dependencies.

Two hard constraints bound the choice. ADR 0001: the tool is deterministic by construction and
holds no LLM/ML — a scan whose verdict depends on a model breaks that. ADR 0010: the leak gate
must **fail closed and reproducibly**, so `cargo test` (and a publish decision) gives the same
answer on every machine. An ML PII detector satisfies neither. Shelling out to gitleaks would add
an external Go binary the pure-Rust tool must find on `PATH`, with its own ruleset that changes
between versions — trading reproducibility and the single-language build for coverage.

## Decision

We will **deepen the deterministic scan rather than adopt a model or an external binary**:

- **A curated, gitleaks-style regex ruleset.** Expand beyond the starter four to the high-signal,
  low-false-positive provider patterns (Stripe, GitHub/GitLab tokens, Google/GCP, Slack, JWT, SSH
  private keys, generic `Bearer`/authorization secrets), each with a class label. The set stays
  **versioned** (ADR 0010) and lives in the tool.
- **Shannon-entropy detection for generic high-entropy secrets**, scoped to assignment *values*
  (a `key = "…"` right-hand side above a length threshold), not free prose — this is how gitleaks
  keeps entropy rules quiet. Entropy is computed in-tool (no dependency); a value above the
  threshold on a secret-shaped assignment is a leak.
- **Deterministic PII only:** email, phone number, Brazilian CPF (with the check-digit validated,
  not a bare 11-digit match), and payment-card numbers **validated by the Luhn algorithm** (so an
  arbitrary 16-digit string does not fire). Each is exact, reproducible arithmetic/regex.
- **Explicitly rejected:** an ML/NER PII detector (breaks ADR 0001 determinism, heavy deps) and
  shelling out to gitleaks/trufflehog (breaks pure-Rust reproducibility, adds an external
  binary/`PATH` dependency). Both were weighed and declined for the determinism boundary.
- **Masking is preserved and extended** (ADR 0010): every new class masks its matched value before
  it reaches any report — a gate that echoed a secret would itself be a leak vector.

The check-digit / Luhn validation is the load-bearing choice for PII noise: it is what separates a
real CPF/card from an incidental number in prose, keeping the fail-closed gate usable.

## Consequences

**Easier / gained:**
- Materially better coverage (dozens of provider secrets + entropy + validated PII) while staying
  100% Rust, deterministic, and reproducible from `cargo` alone.
- No new runtime/tooling dependency beyond a regex engine; no external binary on `PATH`; no model.
- The privacy verdict remains a reproducible fitness function, not a model's guess.

**Harder / accepted trade-offs:**
- The ruleset is ours to maintain and version — new providers/token shapes need patterns added
  (the accepted cost of not delegating to gitleaks' maintained set).
- Entropy and regex will still miss a novel secret shape and can false-positive; the scan stays a
  **backstop**, not the primary control. The primary privacy control is the visibility allowlist
  (ADR 0009) — a private doc never reaches the bundle in the first place.
- CPF is region-specific; other national IDs are future additions when a corpus needs them.

**Follow-ups:**
- The ruleset expansion + entropy (secrets) and the deterministic PII (phone/CPF/Luhn) each ship as
  their own vertical slice with tests.
- A future ADR may add national-ID patterns or tune entropy thresholds if the corpus demands it.

## Verification

**Implementation impact:** `living-docs-core/src/commands/leak_gate.rs` (the expanded ruleset,
the entropy detector, and the deterministic PII validators), building on ADR 0010's leak-gate
command.

**Verification criteria:**
- The gate fires on each added provider-secret shape and on a high-entropy secret-assignment value,
  and does NOT fire on ordinary prose or a git SHA in prose. — fitness function (tests).
- The gate fires on a valid CPF (check-digit) and a Luhn-valid card number, and does NOT fire on an
  arbitrary 11- or 16-digit number that fails the check. — fitness function (tests).
- Every matched value stays masked in the reported message (the raw secret/PII is never printed). —
  fitness function (test).
- `living-docs check docs` stays green over `docs/` with this ADR indexed.
