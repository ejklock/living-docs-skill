---
type: Research
title: Worldwide PII detection catalog — deterministic regex + checksum reference
description: A source-cited catalog of ~70 PII identifiers across 20+ countries (Brazil in depth) with regex + check-digit validation algorithms and a tiering by validation strength, compiled to drive the deterministic leak-gate PII layer. No ML, no external binaries.
status: Accepted
tags: [brazil, checksum, leak-prevention, methodology, pii, privacy, research]
timestamp: 2026-07-17T23:55:00Z
---

# Worldwide PII Detection Catalog — Deterministic (regex + checksum) Reference

Compiled: 2026-07-17
Scope: identifiers a pure-Rust, regex + check-digit scanner can detect (no ML, no external binaries).
Primary source corpus: Microsoft Presidio predefined recognizers (canonical open-source PII set), cross-verified against official standards (ISO/IEC 7064, ISO 13616, ICAO Doc 9303, ISO/IEC 7812/Luhn) and, for Brazilian documents, Receita Federal / TSE / DATASUS / DENATRAN specifications and independent implementations.

---

## 1. Executive summary

### The landscape

This catalog compiles **~70 distinct PII identifiers** across **20+ countries** plus the generic/global family (email, IP, MAC, cards, IBAN, crypto). The backbone is Microsoft Presidio's `predefined_recognizers` tree, which as of 2026 ships recognizers for: Australia, Canada, Finland, Germany (13 recognizers), India, Italy, Korea, Nigeria, Philippines, Poland, Singapore, South Africa, Spain, Sweden, Thailand, Turkey, UK, US — plus a `generic` family (credit card, crypto, email, IBAN, IP, MAC, phone, URL, date). **Presidio ships no Brazilian recognizer**, so the Brazilian set below is sourced independently from Receita Federal, TSE, DATASUS, and DENATRAN specs.

**gitleaks contributes nothing to this catalog.** A full scan of `config/gitleaks.toml` confirms it is exclusively secret/token-focused (API keys, access tokens, private keys). The word "Personal" appears only inside "Personal Access Token" descriptions. It has zero national-ID / SSN / passport / card PII rules. The provider-secret patterns are already covered by the tool's existing secret layer.

### The central methodological insight: checksum is what separates PII from noise

The single most important design principle for a deterministic PII scanner is **tiering by validation strength**:

- **Checksum-validated identifiers are high-signal.** A CPF, a credit card (Luhn), an IBAN (mod-97), a PESEL, or a South African ID is not just "11 digits" — it carries a self-consistent check digit. Random 11-digit numbers pass the *regex* but fail the *checksum* ~90% of the time (for a single mod-11 digit, ~10/11 of random candidates fail; for two check digits, ~99% fail). The checksum is the false-positive filter. **These should ship first and can be flagged with high confidence even without surrounding context words.**

- **Regex-only identifiers are false-positive-prone.** Brazilian RG (no universal checksum), CEP (postal, no checksum), US SSN (no checksum — only structural constraints), UK NINO (structural only), Brazilian passport (`[A-Z]{2}\d{6}`), Singapore NRIC (Presidio does not implement its checksum), India PAN (no check-digit validation in Presidio). These match far too much incidental text (any 8-digit number "looks like" a CEP). They **must be gated by context words** (proximity to "CPF", "RG", "passaporte", "SSN", "social security") and/or offered as opt-in.

- **Structural-constraint identifiers are the middle ground.** US SSN has no checksum but has hard *invalidation* rules (area ≠ 000/666/900-999, group ≠ 00, serial ≠ 0000, not a known placeholder). South African ID embeds a valid birth date AND a Luhn digit. These raise precision meaningfully without a true checksum.

Concretely, Presidio encodes this exact idea in its confidence scores: a bare 9-digit US SSN pattern scores **0.05** ("very weak"), while a dash-formatted one with context scores **0.5** ("medium"), and any checksum-passing card/IBAN is promoted to **MAX_SCORE (1.0)**. Our tiering below mirrors that gradient.

### Key implementation caveat for Rust

The Rust `regex` crate **does not support look-around** (lookbehind/lookahead) or backreferences. Several Presidio patterns rely on them:
- IBAN uses `(?<![A-Z0-9])...(?![A-Z0-9])` — replace with explicit word boundaries / byte-boundary checks in code.
- Credit card uses a negative lookahead `(?!1\d{12}(?!\d))` — replace with post-match length/prefix logic.
- Canada SIN uses a backreference `\1` to force matching delimiters — replace by capturing the delimiter and comparing in code, or enumerate both `-` and ` ` variants.
Prefer: match a permissive `\b`-anchored regex, then apply constraints + checksum in Rust code (which is exactly Presidio's `validate_result` / `invalidate_result` split).

---

## 2. Master catalog table

Regexes are Rust-`regex`-flavored (double-escaped backslashes shown singly here; `\b` = word boundary; `\d` = digit). Where a pattern needs look-around, the "note" column flags it. FP = false-positive risk.

### 2.1 Brazil (sourced independently — not in Presidio)

| Identifier | PII category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|
| CPF | national taxpayer (individual) | `\b\d{3}\.?\d{3}\.?\d{3}-?\d{2}\b` | mod-11, two check digits (weights 10..2 then 11..2); reject 11 equal digits | **low** | Receita Federal; Campus Code; Macoratti |
| CNPJ (numeric) | company taxpayer | `\b\d{2}\.?\d{3}\.?\d{3}/?\d{4}-?\d{2}\b` | mod-11, two check digits (weights 5,4,3,2,9,8,7,6,5,4,3,2 then prepend 6) | **low** | Receita Federal; Campus Code |
| CNPJ (alphanumeric, 2026+) | company taxpayer | `\b[0-9A-Z]{2}\.?[0-9A-Z]{3}\.?[0-9A-Z]{3}/?[0-9A-Z]{4}-?\d{2}\b` | mod-11 over `ASCII(ch)-48`; DVs stay numeric | **low** | IN RFB 2.229/2024; SERPRO DV PDF |
| PIS/PASEP/NIT/NIS | social-security worker id | `\b\d{3}\.?\d{5}\.?\d{2}-?\d{1}\b` | mod-11, single digit, weights 3,2,9,8,7,6,5,4,3,2 | **low** | Caixa/INSS; Macoratti |
| Título de eleitor | voter registration | `\b\d{10,12}\b` (9-digit seq in SP/MG → up to 12) | mod-11 two DVs + UF code 01-28 | **low** | TSE Res. 21.538/2003; OBMEP |
| CNH | driver license | `\b\d{11}\b` | mod-11 chained two DVs (weights 9..1 then 1..9) | **low** | DENATRAN/CONTRAN; siga0984 |
| CNS / Cartão SUS | health id | `\b[1-9]\d{14}\b` | mod-11 weighted sum ×(15..1) ≡ 0 (mod 11); prefix 1/2 definitive vs 7/8/9 provisional | **low** | Portaria MS 940/2011; DATASUS |
| RENAVAM | vehicle registration | `\b\d{11}\b` | mod-11 single DV (weights 3,2,9,8,7,6,5,4,3,2) | med | DENATRAN; siga0984 |
| RG (generic) | identity card | `\b\d{1,2}\.?\d{3}\.?\d{3}-?[0-9Xx]\b` | **none universal** (per-state); SP-RG has mod-11 DV | **high** | per-state SSP; note only |
| CEP | postal code | `\b\d{5}-?\d{3}\b` | **none** | **high** | Correios |
| Passport (BR) | passport | `\b[A-Z]{2}\d{6}\b` | **none** (ICAO MRZ has a check digit, not in the printed number) | med | ICAO Doc 9303 |
| Phone (BR) | phone | `\b(?:\+?55\s?)?\(?\d{2}\)?\s?9?\d{4}-?\d{4}\b` | structural (DDD 11-99, 9 prefix for mobile) | med | ANATEL / E.164 |

### 2.2 United States (Presidio)

| Identifier | Category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|
| SSN | social security | `\b\d{3}[- .]\d{2}[- .]\d{4}\b` (medium) / `\b\d{9}\b` (weak) | **no checksum**; invalidate if area 000/666, group 00, serial 0000, all-same, or known placeholder (078-05-1120 etc.) | med | Presidio `us_ssn` |
| ITIN | taxpayer id | `\b9\d{2}[- ](5\d\|6[0-5]\|7\d\|8[0-8]\|9([0-2]\|[4-9]))[- ]\d{4}\b` | structural only (9xx, group range) | med | Presidio `us_itin` |
| EIN | employer id | `\b\d{2}-\d{7}\b` | **none** (prefix table only) | high | IRS |
| Bank routing (ABA) | financial | `\b\d{9}\b` | ABA checksum mod-10 weights 3,7,1 | low | Presidio `aba_routing` |
| NPI | medical provider | `\b\d{10}\b` | Luhn with `80840` prefix | low | Presidio `us_npi` |
| MBI (Medicare) | health id | 11-char alnum, no S/L/O/I/B/Z | structural | med | Presidio `us_mbi` |
| Driver license | driver license | per-state regex | none | high | Presidio `us_driver_license` |
| Passport | passport | `\b[0-9A-Z]{6,9}\b` | none | high | Presidio `us_passport` |

### 2.3 UK (Presidio)

| Identifier | Category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|
| NHS number | health id | `\b\d{3}[- ]?\d{3}[- ]?\d{4}\b` | **mod-11**: Σ(digit×weight 10..2) then `total % 11 == 0` (weighted so check digit makes sum ≡ 0) | low | Presidio `uk_nhs` |
| NINO | national insurance | `\b[A-CEGHJ-PR-TW-Z][A-CEGHJ-NPR-TW-Z] ?\d{2} ?\d{2} ?\d{2} ?[A-D]\b` (exclude prefixes BG,GB,NK,KN,NT,TN,ZZ) | structural only | med | Presidio `uk_nino` |
| UTR | tax | `\b\d{10}\b` | mod-11 (HMRC) | med | HMRC |
| Postcode | postal | `\b[A-Z]{1,2}\d[A-Z\d]? ?\d[A-Z]{2}\b` | none | high | Presidio `uk_postcode` |
| Driving licence | driver license | 16/18-char pattern | none | med | Presidio `uk_driving_licence` |

### 2.4 EU / other national IDs (Presidio)

| Identifier | Country | Category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|---|
| NIF / DNI | Spain | national id | `\b\d?\d{7}-?[A-Z]\b` | check letter = `"TRWAGMYFPDXBNJZSQVHLCKE"[number % 23]` | low | Presidio `es_nif` |
| NIE | Spain | foreigner id | `\b[XYZ]\d?\d{7}-?[A-Z]\b` | same table; X/Y/Z→0/1/2 prefix then `% 23` | low | Presidio `es_nie` |
| Codice Fiscale | Italy | fiscal code | 16-char alnum (complex, see appendix note) | check char via odd/even weight maps mod 26 → letter A-Z | low | Presidio `it_fiscal_code` |
| PESEL | Poland | national id | `\b\d{2}([02468][1-9]\|[13579][012])(0[1-9]\|[12]\d\|3[01])\d{5}\b` | **mod-10** weights 1,3,7,9,1,3,7,9,1,3; check=(10−sum%10)%10 | low | Presidio `pl_pesel` |
| NIP | Poland | tax | `\b\d{10}\b` | weighted mod-11 (weights 6,5,7,2,3,4,5,6,7) | low | GUS/MF |
| REGON | Poland | business | `\b\d{9}\b` or `\d{14}` | mod-11 | low | GUS |
| BSN | Netherlands | citizen id | `\b\d{9}\b` | **11-proef**: Σ(d_i×(9..2)) − last×1 ≡ 0 (mod 11) | low | Dutch gov |
| NIF | Portugal | tax | `\b\d{9}\b` | mod-11 (weights 9..2), check = 11−rem (≥10→0) | low | AT Portugal |
| PPS | Ireland | personal id | `\b\d{7}[A-W][A-IW]?\b` | mod-23 check letter | low | Irish Revenue |
| Steuer-ID | Germany | tax id | `\b[1-9]\d{10}\b` | **ISO 7064 Mod 11,10**; exactly one repeated digit rule | low | Presidio `de_tax_id` |
| Personnummer | Sweden | national id | `\b(\d{2})?\d{6}[-+]?\d{4}\b` | **Luhn** over 10 digits | low | Presidio `se_personnummer` |
| Organisationsnummer | Sweden | business | `\b\d{6}-?\d{4}\b` | Luhn | low | Presidio `se_organisationsnummer` |
| Henkilötunnus | Finland | personal id | `\b\d{6}[-+A-Y]\d{3}[0-9A-Y]\b` | control char = `"0123456789ABCDEFHJKLMNPRSTUVWXY"[int(ddmmyy+nnn) % 31]` | low | Presidio `fi_personal_identity_code` |

### 2.5 Asia-Pacific / Africa (Presidio)

| Identifier | Country | Category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|---|
| Aadhaar | India | national id | `\b[2-9]\d{3}[- :]?\d{4}[- :]?\d{4}\b` | **Verhoeff** checksum; first digit ≥2; reject palindrome | low | Presidio `in_aadhaar` |
| PAN | India | tax | `\b[A-Z]{5}\d{4}[A-Z]\b` (4th char ∈ ABCFGHJLPT) | **none** (Presidio does not implement the check digit) | med | Presidio `in_pan` |
| GSTIN | India | tax | 15-char alnum | mod-36 check char | low | Presidio `in_gstin` |
| Voter ID | India | voter | `\b[A-Z]{3}\d{7}\b` | none | high | Presidio `in_voter` |
| NRIC / FIN | Singapore | national/foreigner id | `\b[STFGM]\d{7}[A-Z]\b` | **Presidio: regex only** (real checksum is weighted mod-11 → letter table) | med | Presidio `sg_fin` |
| UEN | Singapore | business | multiple formats | structural | med | Presidio `sg_uen` |
| ID number | South Africa | national id | `\b\d{10}[0-2][89]\d\b` | **Luhn** + valid embedded birth date + citizenship digit 0/1/2 | low | Presidio `za_id_number` |
| TFN | Australia | tax file number | `\b\d{3} \d{3} \d{3}\b` / `\b\d{9}\b` | **mod-11** weights [1,4,3,7,5,8,6,9,10], sum ≡ 0 | low | Presidio `au_tfn` |
| ABN | Australia | business | `\b\d{2} \d{3} \d{3} \d{3}\b` | **mod-89** weights [10,1,3,5,7,9,11,13,15,17,19] (subtract 1 from first digit), ≡ 0 | low | Presidio `au_abn` |
| ACN | Australia | company | `\b\d{3} \d{3} \d{3}\b` | **mod-10** complement, weights [8,7,6,5,4,3,2,1] | low | Presidio `au_acn` |
| Medicare | Australia | health | `\b[2-6]\d{3} \d{5} \d\b` | **mod-10** weights [1,3,7,9,1,3,7,9] on first 8, == 9th digit | low | Presidio `au_medicare` |
| TIN | Philippines | tax | `\b\d{3}-\d{3}-\d{3}(-\d{3,5})?\b` | structural | med | Presidio `ph_tin` |
| UMID | Philippines | national id | `\b\d{4}-\d{7}-\d\b` | structural | med | Presidio `ph_umid` |
| TNIN | Thailand | national id | `\b\d{13}\b` | **mod-11**: Σ(d_i×(13..2)), check=(11−sum%11)%10 | low | Presidio `th_tnin` |
| National ID | Turkey | national id | `\b[1-9]\d{10}\b` | dual checksum (10th and 11th digits) | low | Presidio `tr_national_id` |
| RRN | Korea | resident reg | `\b\d{6}-?[1-4]\d{6}\b` | weighted mod-11 | low | Presidio `kr_rrn` |
| NIN | Nigeria | national id | `\b\d{11}\b` | none | high | Presidio `ng_nin` |

### 2.6 Generic / global (Presidio `generic`)

| Identifier | Category | Regex (Rust) | Validation | FP | Source |
|---|---|---|---|---|---|
| Credit/debit card | financial | `\b(?:4\d{3}\|5[0-5]\d{2}\|6\d{3}\|3\d{3})[- ]?\d{3,4}[- ]?\d{3,4}[- ]?\d{3,5}\b` | **Luhn** (mod-10) | low (with Luhn) | Presidio `credit_card`; ISO/IEC 7812 |
| — Visa | | `\b4\d{12}(\d{3})?\b` | Luhn | low | issuer prefix |
| — Mastercard | | `\b(5[1-5]\d{2}\|2[2-7]\d{2})\d{12}\b` | Luhn | low | issuer prefix |
| — Amex | | `\b3[47]\d{13}\b` | Luhn | low | issuer prefix |
| — Discover | | `\b6(?:011\|5\d{2}\|4[4-9]\d)\d{12,15}\b` | Luhn | low | issuer prefix |
| — Diners | | `\b3(?:0[0-5]\|[68]\d)\d{11}\b` | Luhn | low | issuer prefix |
| — JCB | | `\b35\d{14}\b` | Luhn | low | issuer prefix |
| IBAN | financial | `\b[A-Z]{2}\d{2}(?:[ -]?[A-Z0-9]{4}){2,7}[A-Z0-9]{0,3}\b` | **ISO 13616 mod-97** == 1 + per-country length | low | Presidio `iban`; ISO 13616 |
| SWIFT/BIC | financial | `\b[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?\b` | structural (ISO 9362) | med | ISO 9362 |
| Email | contact | `\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b` | structural | low | Presidio `email` |
| IPv4 | network | `\b(?:(?:25[0-5]\|2[0-4]\d\|1?\d?\d)\.){3}(?:25[0-5]\|2[0-4]\d\|1?\d?\d)\b` | octet range | med | Presidio `ip` |
| IPv6 | network | (full RFC 5952 alternation) | structural | med | Presidio `ip` |
| MAC | network | `\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b` | structural | med | Presidio `mac` |
| Phone (E.164) | contact | `\b\+[1-9]\d{6,14}\b` | length + country prefix | med | E.164; Presidio `phone` |
| Bitcoin address | crypto (opt) | `\b(bc1[a-z0-9]{25,90}\|[13][a-km-zA-HJ-NP-Z1-9]{25,34})\b` | Base58Check / Bech32 | med | Presidio `crypto` |
| Ethereum address | crypto (opt) | `\b0x[a-fA-F0-9]{40}\b` | EIP-55 checksum (mixed-case) | med | EIP-55 |

---

## 3. Appendix — Brazilian check-digit algorithms (implementation-ready)

All Brazilian algorithms below are **mod-11 variants** except where noted. Rule of thumb for a mod-11 check digit: `resto = soma % 11; DV = (resto < 2) ? 0 : 11 - resto`. Reject inputs that are all-identical digits *before* running the algorithm.

### 3.1 CPF (11 digits, two check digits)

Input: 9 base digits `d1..d9`, produce `dv1`, `dv2`.

**DV1:**
1. Multiply `d1..d9` by weights `10,9,8,7,6,5,4,3,2`.
2. `soma = Σ`; `resto = soma % 11`.
3. `dv1 = (resto < 2) ? 0 : 11 - resto`.

**DV2:**
1. Take `d1..d9` **plus dv1** (10 digits), weights `11,10,9,8,7,6,5,4,3,2`.
2. `resto = soma % 11`; `dv2 = (resto < 2) ? 0 : 11 - resto`.

Worked example `111.444.777-XX`: DV1 → sum 162, 162%11 = 8, 11−8 = **3**. DV2 → sum 204, 204%11 = 6, 11−6 = **5**. Result `111.444.777-35`.
Reject the 10 forbidden repeated-digit CPFs (`00000000000`…`99999999999`) — they pass the math but Receita rejects them.
Sources: Campus Code; Macoratti `alg_cpf.htm`.

### 3.2 CNPJ numeric (14 digits, two check digits)

Input: 12 base digits, produce `dv1`, `dv2`.

**DV1:** weights over the 12 base digits = `5,4,3,2,9,8,7,6,5,4,3,2`. `resto = soma % 11`; `dv1 = (resto < 2) ? 0 : 11 - resto`.
**DV2:** weights over the 13 digits (12 base + dv1) = `6,5,4,3,2,9,8,7,6,5,4,3,2`. Same rest rule.

Worked example `59.541.264/0001-XX`: DV1 sum 177, 177%11 = 1, 11−1 = 10 → **0**. DV2 sum 206, 206%11 = 8, 11−8 = **3** → `...-03`.
Sources: Campus Code; Receita Federal.

### 3.3 CNPJ alphanumeric (2026+, IN RFB 2.229/2024)

Structure: positions 1-12 accept `0-9` and `A-Z` (uppercase, no accents); positions 13-14 (the DVs) **stay numeric**. Same length (14).

**Character value:** `valor(ch) = ASCII(ch) − 48`. So `'0'..'9'` → `0..9`; `'A'..'Z'` → `17..42` (`A=65−48=17`). This is retro-compatible: numeric CNPJs compute identically.

**DV1:** convert the 12 chars via `ASCII−48`; apply weights `5,4,3,2,9,8,7,6,5,4,3,2` (equivalently weights 2..9 right-to-left restarting after the 8th). `resto = soma % 11`; `dv1 = (resto ∈ {0,1}) ? 0 : 11 − resto`.
**DV2:** append dv1 (13 chars), weights `6,5,4,3,2,9,8,7,6,5,4,3,2`. Same rule.

SERPRO worked example `12ABC34501DE`: values `1 2 17 18 19 3 4 5 0 1 20 21` (+ dv1 for DV2); weighted sum for DV2 stage = 424; final document `12.ABC.345/01DE-35`. Receita recommends avoiding letters `I, O, Q, F` (visual/collision issues).
Sources: SERPRO `calculodvcnpjalfanaumerico.pdf`; Receita Federal `cnpj-alfanumerico.pdf`; IN RFB 2.229/2024.

### 3.4 PIS / PASEP / NIT / NIS (11 digits, one check digit)

Input: 10 base digits, produce `dv`.
1. Weights `3,2,9,8,7,6,5,4,3,2` over the 10 base digits.
2. `resto = soma % 11`; `dv = (resto < 2) ? 0 : 11 − resto` (equivalently: if `11−resto` is 10 or 11 → 0).
Sources: Macoratti `alg_pis.htm`; Caixa/INSS.

### 3.5 Título de eleitor (12 digits: 8-seq + 2 UF + 2 DV)

Structure: `SSSSSSSS` (sequential, 8 digits; **9 in SP/MG**) + `UU` (UF code 01-28) + `DV1 DV2`.

**DV1:** multiply the sequential digits left-to-right by `2,3,4,5,6,7,8,9`; `resto = soma % 11`; `DV1 = resto`. If `resto == 10` → `DV1 = 0`. **Special: in SP (01) and MG (02), if `resto == 0` → `DV1 = 1`.**
**DV2:** multiply `[UF_dig1, UF_dig2, DV1]` by `7,8,9`; `resto = soma % 11`; `DV2 = resto`; `10→0`; SP/MG `0→1`.

UF table (positions 9-10): 01 SP · 02 MG · 03 RJ · 04 RS · 05 BA · 06 PR · 07 CE · 08 PE · 09 SC · 10 GO · 11 MA · 12 PB · 13 PA · 14 ES · 15 PI · 16 RN · 17 AL · 18 MT · 19 MS · 20 DF · 21 SE · 22 AM · 23 RO · 24 AC · 25 AP · 26 RR · 27 TO · 28 Exterior (ZZ).
Worked: `1023 8501 06` → DV1 sum 117, 117%11 = 7 → DV1 **7**. Result e.g. `...06 7Y`.
Sources: OBMEP "A Matemática nos Documentos"; TSE Resolução 21.538/2003.

### 3.6 CNH (11 digits, two chained check digits)

Input: 9 base digits `d1..d9`.

**DV1:** weights `9,8,7,6,5,4,3,2,1` (i.e. `d1×9 … d9×1`). `soma`; `resto = soma % 11`. If `resto >= 10` → `dv1 = 0` and set correction flag `x = 2`; else `dv1 = resto`, `x = 0`.
**DV2:** weights `1,2,3,4,5,6,7,8,9` (`d1×1 … d9×9`). `soma`; `resto = soma % 11`. Apply correction: if `resto - x < 0` handling per the DENATRAN routine; if `resto >= 10` → `dv2 = 0`; else `dv2 = resto - x` (guarded ≥0).
Note: implementations vary in how the correction factor is applied; validate against a known-valid corpus. Reject all-equal-digit strings.
Sources: siga0984 "Validação de CNH"; DENATRAN/CONTRAN.

### 3.7 CNS / Cartão SUS (15 digits)

Two regimes by first digit:

**Definitive (prefix 1 or 2):** derived from the PIS. Take the 11-digit PIS base; `soma = Σ(pis_i × (15..5))`; `resto = soma % 11`; `dv = 11 − resto`; if `dv == 11` → `dv = 0`; if `dv == 10` → add 2 to soma, recompute, and the suffix becomes `"001"+dv`, else suffix `"000"+dv`. The full 15-digit number then satisfies the unified check below.

**Provisional (prefix 7, 8, or 9):** `Σ(d_i × (15..1)) % 11 == 0` over all 15 digits.

**Unified validator (covers both):** accept if it matches `^[1-2]\d{10}00[0-1]$` **or** `^[7-9]\d{14}$`, then require `Σ(d_i × (15 − i)) % 11 == 0` for `i = 0..14`.
Sources: DATASUS reference routine; Portaria MS 940/2011; Yanaga; GeraValida.

### 3.8 RENAVAM (11 digits, one check digit)

Since 2013: 11 digits (old 9-digit numbers left-zero-padded). Base = first 10 digits, DV = 11th.
1. Weights `3,2,9,8,7,6,5,4,3,2` over the 10 base digits (right-to-left equivalent).
2. `soma`; `dv = (soma * 10) % 11`; if `dv == 10` → `dv = 0`. (Equivalent to standard `resto`/`11−resto` mod-11 form.)
Sources: siga0984 "Validação de RENAVAM"; SEFAZ-PR módulo 11 doc.

### 3.9 RG (state-issued — no universal checksum)

There is **no nationwide RG check-digit algorithm**; each SSP (state) issues its own format. **São Paulo RG** carries a mod-11 check digit (last position, can be `X` for value 10): weights `2..9` over the 8 base digits, `resto = soma % 11`, `DV = 11 − resto` (`10→X`, `11→0`). Treat RG as **regex-only, context-gated, high-FP** except when the SP algorithm is explicitly enabled.

---

## 4. International checksum algorithm reference (cross-verified)

| Algorithm | Used by | Mechanics | Standard |
|---|---|---|---|
| **Luhn (mod-10)** | payment cards, Canada SIN, South Africa ID, US NPI, Sweden personnummer | double every 2nd digit from right, subtract 9 if >9, sum ≡ 0 (mod 10) | ISO/IEC 7812-1 |
| **mod-97** | IBAN | move first 4 chars to end, letters→digits (A=10..Z=35), integer mod 97 == 1 | ISO 13616 / ISO 7064 mod-97,10 |
| **ISO 7064 Mod 11,10** | Germany Steuer-ID | iterative product chain, final `11 − product` (10→0) | ISO/IEC 7064 |
| **mod-11 (weighted)** | CPF, CNPJ, PIS, CNH, CNS, RENAVAM, título, UK NHS, NL BSN, PT NIF, PL NIP, TH TNIN, AU TFN | Σ(digit×weight) mod 11 → check digit | (per-doc spec) |
| **mod-23 letter** | Spain NIF/NIE, Ireland PPS | `number % 23` indexes a 23-letter table | national spec |
| **mod-26 char** | Italy Codice Fiscale | odd/even positional weight maps summed mod 26 → A-Z | national spec |
| **Verhoeff** | India Aadhaar | dihedral-group D5 tables (d, p, inv); trailing check ≡ 0 | Verhoeff 1969 |
| **Damm** | (alternative to Verhoeff; not seen in this corpus) | quasigroup table, single-digit | Damm 2004 |
| **ICAO check digit** | passport MRZ fields | weights `7,3,1` repeating, letters A=10..Z=35, mod 10 | ICAO Doc 9303 |
| **ISO/IEC 7064** | umbrella standard for the above pure check-digit systems | Mod 11,10 / Mod 97,10 / Mod 37,36 etc. | ISO/IEC 7064:2003 |

Note on ICAO Doc 9303: the check digit lives in the **machine-readable zone (MRZ)**, computed over the document-number field with weights 7,3,1. The plain printed passport number (e.g. Brazil `[A-Z]{2}\d{6}`) has **no standalone checksum**, so passport regexes are context-dependent.

---

## 5. References

All accessed 2026-07-17.

**Primary recognizer corpus**
- Microsoft Presidio, `presidio-analyzer/presidio_analyzer/predefined_recognizers/` — https://github.com/microsoft/presidio — backed every non-Brazil regex + `validate_result`/`invalidate_result` checksum logic (credit card, IBAN, US SSN/ITIN, CA SIN, UK NHS/NINO, ES NIF/NIE, IT fiscal code, PL PESEL, IN Aadhaar/PAN, SG FIN, ZA ID, AU TFN/ABN/ACN/Medicare, DE Steuer-ID, FI hetu, etc.).
- gitleaks, `config/gitleaks.toml` — https://github.com/gitleaks/gitleaks — reviewed and found to contain **no PII identifier rules** (secret/token-only); contributes nothing beyond existing provider-secret coverage.

**Standards**
- ISO/IEC 7812-1 (Luhn / payment card numbering) — backed card + SIN + ZA ID + NPI checks.
- ISO 13616 / ISO/IEC 7064 mod-97,10 (IBAN) — backed IBAN mod-97 and per-country length.
- ISO/IEC 7064:2003 (check-character systems) — backed DE Steuer-ID Mod 11,10 and the algorithm umbrella.
- ICAO Doc 9303 (MRTD) — backed passport MRZ check-digit note (weights 7,3,1).

**Brazil**
- Receita Federal — CPF/CNPJ rules; CNPJ alfanumérico Q&A PDF — https://www.gov.br/receitafederal/pt-br/centrais-de-conteudo/publicacoes/perguntas-e-respostas/cnpj/cnpj-alfanumerico.pdf — backed CPF/CNPJ + 2026 alphanumeric structure.
- IN RFB nº 2.229/2024 + SERPRO, "Cálculo dos DV de CNPJ alfanumérico" — https://www.serpro.gov.br/menu/noticias/videos/calculodvcnpjalfanaumerico.pdf — backed ASCII−48 conversion + worked example `12ABC34501DE`.
- Campus Code, "O cálculo do dígito verificador do CPF e do CNPJ" — https://www.campuscode.com.br/conteudos/o-calculo-do-digito-verificador-do-cpf-e-do-cnpj — second independent source for CPF/CNPJ weights + worked examples.
- Macoratti, `alg_cpf.htm` / `alg_pis.htm` — https://www.macoratti.net/alg_cpf.htm — CPF and PIS/PASEP weight sequences.
- TSE, Resolução nº 21.538/2003; OBMEP "A Matemática nos Documentos: Título de Eleitor" — https://clubes.obmep.org.br/blog/a-matematica-nos-documentos-titulo-de-eleitor/ — backed título de eleitor DV algorithm + UF table + SP/MG special rule.
- siga0984 "Tudo em AdvPL" — CNH: https://siga0984.wordpress.com/2019/05/01/algoritmos-validacao-de-cnh/ ; RENAVAM: https://siga0984.wordpress.com/2019/05/01/algoritmos-validacao-de-renavam/ — backed CNH chained DV and RENAVAM DV.
- Portaria MS nº 940/2011; DATASUS reference routine (via Yanaga, GeraValida) — http://www.yanaga.com.br/2012/06/validacao-do-cns-cartao-nacional-de.html — backed CNS definitive/provisional mod-11 and unified validator.

**International national IDs (secondary confirmation)**
- Wikipedia PESEL / Aadhaar / South African identity number — linked from Presidio recognizer docstrings; used to confirm PESEL mod-10, Aadhaar Verhoeff, ZA Luhn+date.

---

## 6. Implementation recommendations — tiering for the deterministic scanner

### Tier 1 — checksum-validated, ship first (flag with high confidence, context optional)

These have a real check digit; a match that passes the checksum is almost certainly PII.

- **Brazil:** CPF, CNPJ (numeric **and** alphanumeric), PIS/PASEP/NIT, Título de eleitor, CNH, CNS/Cartão SUS, RENAVAM.
- **Financial (global):** credit/debit cards (Luhn) by issuer, IBAN (mod-97), US ABA routing, US NPI.
- **International IDs:** Spain NIF/NIE, Italy Codice Fiscale, Poland PESEL/NIP, Netherlands BSN, Portugal NIF, Germany Steuer-ID, Sweden personnummer/organisationsnummer, Finland hetu, India Aadhaar, South Africa ID, Australia TFN/ABN/ACN/Medicare, Thailand TNIN, Turkey national ID, Korea RRN, UK NHS.

Masking: mask all but last 2-4 chars (e.g. `***.***.**9-05` for CPF, `**** **** **** 1234` for cards). Never log the raw value even on a match.

### Tier 2 — regex + mandatory context word (medium FP without a checksum)

Structural constraints only; require a nearby label to fire.

- **US SSN** (area/group/serial constraints + placeholder blocklist), **ITIN**, **UK NINO**, **Singapore NRIC/FIN** (until its mod-11 letter check is implemented — recommend implementing it to promote to Tier 1), **India PAN** (implement the check digit to promote), **Ireland PPS**, **US EIN**, **SWIFT/BIC**, **E.164 phone**, **Brazilian phone**.

Context words to require (per identifier): CPF/`cpf`, `ssn`/`social security`, `nino`/`national insurance`, `nric`/`fin`, `passaporte`/`passport`, etc. Presidio's `CONTEXT` lists (captured above) are a ready-made seed list.

### Tier 3 — regex-only, opt-in / high FP (off by default)

No checksum and weak structure; will match incidental numbers. Ship behind an explicit flag.

- **Brazil RG** (except SP mod-11 when enabled), **CEP**, **Brazilian/US/UK passport numbers**, **UK postcode**, **US/UK driver license**, **EIN**, **India Voter ID**, **Nigeria NIN**, **email/IPv4/IPv6/MAC** (these are PII-adjacent but extremely common in logs — default to opt-in or low severity), **crypto wallet addresses** (BTC/ETH — optional).

### Cross-cutting engineering notes

1. **Two-stage match** (Presidio's model): permissive `\b`-anchored regex → Rust `validate_result` doing constraint + checksum. Keeps regexes simple and sidesteps the `regex` crate's lack of look-around/backreferences.
2. **Reject trivial inputs before checksum:** all-equal digits, known placeholders (SSN `078-05-1120`, `123-45-6789`; the 10 repeated CPFs). Presidio does this via `invalidate_result`.
3. **Normalize separators** before checksum (strip `.`, `-`, `/`, spaces) — mirror Presidio's `replacement_pairs = [("-", ""), (" ", "")]`.
4. **Confidence gradient, not boolean:** adopt Presidio's scoring — weak regex ~0.3, +context ~0.5, +passing checksum → 1.0. Lets downstream policy choose a threshold.
5. **CNPJ alphanumeric is live as of July 2026** — the scanner must accept `[0-9A-Z]` in positions 1-12 and use the `ASCII−48` conversion; a numeric-only CNPJ validator will silently miss all new registrations.
6. **CNS dual regime** — a validator that only handles prefix 1/2 rejects ~half of legitimate cards (7/8/9 provisional). Implement the unified validator.
