---
type: ADR
title: Worldwide PII detection is checksum-tiered, two-stage, and deterministic
description: Expand the leak gate's PII scan from a minimal set to a worldwide catalog (~70 identifiers, 20+ countries, Brazil in depth), implemented as a two-stage detector (permissive regex then Rust checksum/constraint) with boolean-by-tier severity — Tier 1 checksum-validated always fires, Tier 2 needs a context word, Tier 3 is opt-in — staying pure-Rust and deterministic, never ML.
status: Accepted
supersedes:
superseded_by:
tags: [security, privacy, pii, publishing, leak-prevention, brazil, methodology]
timestamp: 2026-07-17T23:58:00Z
---

# 0012. Worldwide PII detection is checksum-tiered, two-stage, and deterministic

## Context

ADR 0011 fixed the leak gate's determinism boundary (curated ruleset + Shannon entropy, never
ML) and shipped a minimal PII surface (email). The moat this tool protects is a knowledge corpus
that must not leak client- or person-identifying data on publish, and "email only" is far short
of that. The research note `/research/0001-worldwide-pii-detection-catalog.md` surveyed the field
and compiled a source-cited catalog of ~70 identifiers across 20+ countries, with the exact
check-digit algorithm for each Brazilian document and cross-verified algorithms for the
international ones. The user's decision: implement the **full worldwide catalog**, with Brazil in
depth, with a **boolean (not gradient) gate**.

Three findings from the research bound the design:

1. **The checksum is the false-positive filter.** A CPF, a Luhn card, an IBAN, a PESEL is not
   merely "N digits" — it carries a self-consistent check digit that ~99% of random candidates
   fail (two mod-11 digits). Detection *strength* therefore tiers naturally by whether a validator
   exists, not by how clever the regex is.
2. **Rust `regex` has no look-around or backreferences.** Several canonical patterns (IBAN, card,
   Canada SIN) depend on them. The portable, deterministic answer is Presidio's **two-stage model**:
   a permissive word-boundary regex, then the constraint/checksum in Rust code.
3. **Two live gotchas.** Brazil's **CNPJ went alphanumeric in July 2026** (positions 1–12 accept
   `A–Z` via `ASCII−48`; a numeric-only validator silently misses every new registration), and the
   **CNS/Cartão SUS has a dual regime** (validating only prefix 1/2 rejects ~half the legitimate
   cards). Both are hard requirements, not nice-to-haves.

## Decision

We will expand PII detection into a **worldwide, checksum-tiered, two-stage, deterministic** layer,
sourced from the research catalog, delivered across vertical slices.

- **Two-stage detection.** Each identifier is a permissive `\b`-anchored regex followed by a Rust
  validator (`validate` doing separator-normalization, trivial-input rejection, structural
  constraints, and the checksum). This keeps regexes simple and sidesteps the `regex` crate's lack
  of look-around — the same split Presidio uses (`validate_result` / `invalidate_result`).

- **Boolean severity, tiered by validation strength** (the user's call — the gate stays binary, no
  confidence gradient):
  - **Tier 1 — checksum-validated: always fires.** A match that passes its check digit is almost
    certainly real PII, so it needs no surrounding context. Covers Brazil (CPF, CNPJ numeric **and**
    alphanumeric, PIS/PASEP/NIT, título de eleitor, CNH, CNS/Cartão SUS, RENAVAM), global financial
    (Luhn cards by issuer, IBAN mod-97, US ABA, US NPI), and international IDs with a validator
    (Spain NIF/NIE, Italy Codice Fiscale, Poland PESEL/NIP, Netherlands BSN, Portugal NIF, Germany
    Steuer-ID, Sweden personnummer, Finland hetu, India Aadhaar, South Africa ID, Australia
    TFN/ABN/ACN/Medicare, Thailand, Turkey, Korea, UK NHS).
  - **Tier 2 — structural + mandatory context word: fires only near a label.** No true checksum,
    only structural constraints, so a nearby label ("cpf", "ssn", "passport", "nino", …) is required
    to fire. Covers US SSN/ITIN/EIN, UK NINO, Singapore NRIC/FIN, India PAN, Ireland PPS,
    SWIFT/BIC, phone numbers.
  - **Tier 3 — regex-only: opt-in, off by default.** No checksum and weak structure; matches
    incidental text. Ships behind an explicit flag. Covers RG (except SP's mod-11 when enabled),
    CEP, passport numbers, postcodes, driver licenses, voter IDs, and email/IPv4/IPv6/MAC/crypto.

- **Full coverage delivered in slices.** All three tiers are in scope; because the whole catalog
  exceeds the slicing caps, it ships as a sequence of vertical slices (Brazil Tier 1 first, then
  global financial, then international IDs, then Tier 2 context-gated, then Tier 3 opt-in). Each
  slice extends the shared checksum library and adds detectors + tests.

- **Module architecture.** A `pii` module in `living-docs-core`: `pii/checksum.rs` (reusable,
  exhaustively-tested algorithms — Luhn, weighted mod-11, mod-97, mod-23 letter, Verhoeff,
  ISO 7064 Mod 11,10), region/domain detector files (`brazil.rs`, `financial.rs`,
  `international.rs`, `generic.rs`), and a registry + two-stage runner wired into `leak_gate`'s
  per-doc scan alongside the secret scan. Email stays in the existing secret scan (already covered)
  to avoid double-reporting.

- **Masking and hygiene.** Every match masks all but the last 2–4 characters; the raw value is
  never printed (a gate that echoes PII is itself a leak). Reject trivial inputs *before* the
  checksum (all-equal digits, known placeholders); normalize separators (`.`, `-`, `/`, space).

- **Hard requirements from the gotchas.** CNPJ detection accepts the alphanumeric form with the
  `ASCII−48` conversion; CNS uses the unified validator covering both the definitive (1/2) and
  provisional (7/8/9) regimes.

- **Explicitly rejected:** ML/NER PII detection (breaks ADR 0011's determinism boundary), shelling
  out to external scanners (breaks pure-Rust reproducibility), and a Presidio-style confidence
  gradient (the gate stays boolean-by-tier — simpler and consistent with the existing binary
  leak-gate). Each was weighed against the research and declined.

The catalog note is the single source of truth for the algorithms; the tiering here is the policy
that consumes it.

## Consequences

**Easier / gained:**
- Comprehensive worldwide + deep-Brazil PII coverage, 100% Rust, deterministic, reproducible from
  `cargo` alone — the privacy verdict stays a fitness function, not a model's guess.
- Checksum-tiering keeps the high-signal identifiers noise-free while quarantining the
  false-positive-prone ones behind context words or an opt-in flag.
- The reusable checksum library and two-stage runner make adding a new identifier cheap (a regex +
  a validator + tests), so the catalog can grow with the corpus.

**Harder / accepted trade-offs:**
- The ruleset and algorithms are ours to maintain and version (the accepted cost of not delegating
  to an ML detector or an external binary). New identifiers and spec changes (like CNPJ 2026) are
  our patch to make.
- Tier 2 needs a context-proximity mechanism (a label window around the match) that the current
  scan does not have — added with that slice.
- Regex-crate limitations force the two-stage split everywhere; a few identifiers (IBAN, card,
  SIN) cannot be a single regex.
- The catalog is large; full coverage lands over several slices rather than at once.

**Follow-ups (the slice sequence):**
- Slice: Brazil Tier 1 core (checksum lib + CPF, CNPJ numeric+alphanumeric, PIS).
- Slice: Brazil Tier 1 rest (título, CNH, CNS dual-regime, RENAVAM).
- Slice: global financial Tier 1 (Luhn cards by issuer, IBAN, ABA, NPI).
- Slice(s): international IDs Tier 1 (EU, then APAC/Africa).
- Slice: Tier 2 context-gated identifiers + the proximity mechanism.
- Slice: Tier 3 opt-in identifiers behind the flag.

## Verification

**Implementation impact:** a new `living-docs-core/src/pii/` module (`checksum.rs` + detector
files + registry/runner) wired into `living-docs-core/src/commands/leak_gate.rs`, building on
ADR 0011's leak-gate command and consuming `/research/0001-worldwide-pii-detection-catalog.md`.

**Verification criteria:**
- Each Tier 1 identifier fires the gate on a checksum-valid value and does NOT fire on the same
  value with a broken check digit (the checksum is the discriminator). — fitness function (tests).
- CNPJ fires on a valid alphanumeric registration (not only numeric); CNS fires on both a
  definitive (prefix 1/2) and a provisional (prefix 7/8/9) card. — fitness function (tests).
- A Tier 2 identifier fires only when a context word is within the label window, and stays quiet
  in bare prose. — fitness function (tests).
- Tier 3 identifiers stay quiet by default and fire only under the explicit opt-in flag. — fitness
  function (tests).
- Every match masks all but the last 2–4 characters; the raw value never appears in the report. —
  fitness function (test).
- `living-docs check docs` stays green over `docs/` with this ADR and the research note indexed.
