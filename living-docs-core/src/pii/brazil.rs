//! Brazil Tier 1 detectors (ADR 0012, research note 0001 §3.1-3.8): CPF,
//! CNPJ (numeric and the 2026 alphanumeric form via a single detector),
//! PIS/PASEP/NIT, título de eleitor, CNH, CNS/Cartão SUS, and RENAVAM. Each
//! pairs a permissive regex with a Rust validator that normalizes
//! separators, rejects trivial (all-equal-digit) inputs where applicable,
//! and verifies the document's check digit(s).

use super::checksum;
use regex::Regex;

const CPF_DV1_WEIGHTS: [u32; 9] = [10, 9, 8, 7, 6, 5, 4, 3, 2];
const CPF_DV2_WEIGHTS: [u32; 10] = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2];

/// CPF (11 digits, two check digits): the ten forbidden all-repeated-digit
/// CPFs pass the mod-11 math but are Receita Federal placeholders, so they
/// are rejected before the checksum runs (research note 0001 §3.1).
fn validate_cpf(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    let dv1 = checksum::weighted_mod11_dv(&ds[..9], &CPF_DV1_WEIGHTS);
    if dv1 != ds[9] {
        return false;
    }
    let dv2 = checksum::weighted_mod11_dv(&ds[..10], &CPF_DV2_WEIGHTS);
    dv2 == ds[10]
}

const CNPJ_DV1_WEIGHTS: [u32; 12] = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
const CNPJ_DV2_WEIGHTS: [u32; 13] = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

fn strip_cnpj_separators(matched: &str) -> String {
    matched
        .chars()
        .filter(|c| !matches!(c, '.' | '/' | '-'))
        .collect()
}

fn cnpj_shape_is_valid(chars: &[char]) -> bool {
    if chars.len() != 14 {
        return false;
    }
    let base_is_alnum = chars[..12]
        .iter()
        .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase());
    let dvs_are_digits = chars[12..].iter().all(|c| c.is_ascii_digit());
    base_is_alnum && dvs_are_digits
}

/// `ASCII(ch) - 48` (research note 0001 §3.3, IN RFB 2.229/2024): maps
/// `'0'..'9'` to `0..9` and `'A'..'Z'` to `17..42`, and computes identically
/// for a numeric CNPJ — a single detector retro-covers both forms.
fn cnpj_base_values(chars: &[char]) -> Vec<u32> {
    chars[..12].iter().map(|c| *c as u32 - 48).collect()
}

/// CNPJ, numeric or 2026 alphanumeric (14 chars, two trailing numeric check
/// digits): strips separators, validates shape, rejects an all-equal base,
/// then verifies both mod-11 check digits over the ASCII-48 values.
fn validate_cnpj(matched: &str) -> bool {
    let stripped = strip_cnpj_separators(matched);
    let chars: Vec<char> = stripped.chars().collect();
    if !cnpj_shape_is_valid(&chars) {
        return false;
    }
    let base = cnpj_base_values(&chars);
    if checksum::all_same(&base) {
        return false;
    }
    let Some(dv1_expected) = chars[12].to_digit(10) else {
        return false;
    };
    let dv1 = checksum::weighted_mod11_dv(&base, &CNPJ_DV1_WEIGHTS);
    if dv1 != dv1_expected {
        return false;
    }
    let Some(dv2_expected) = chars[13].to_digit(10) else {
        return false;
    };
    let mut with_dv1 = base;
    with_dv1.push(dv1);
    let dv2 = checksum::weighted_mod11_dv(&with_dv1, &CNPJ_DV2_WEIGHTS);
    dv2 == dv2_expected
}

const PIS_DV_WEIGHTS: [u32; 10] = [3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

/// PIS/PASEP/NIT/NIS (11 digits, one check digit, research note 0001 §3.4).
fn validate_pis(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    let dv = checksum::weighted_mod11_dv(&ds[..10], &PIS_DV_WEIGHTS);
    dv == ds[10]
}

const TITULO_DV1_WEIGHTS: [u32; 8] = [2, 3, 4, 5, 6, 7, 8, 9];

/// `resto` for título de eleitor's DV1/DV2 (research note 0001 §3.5) maps to
/// the check digit directly, with `10 -> 0`; São Paulo (UF 01) and Minas
/// Gerais (UF 02) additionally remap `resto == 0 -> 1` so those states'
/// titles never carry an all-zero-looking check digit.
fn titulo_resto_to_dv(resto: u32, is_sp_or_mg: bool) -> u32 {
    if resto == 10 {
        0
    } else if is_sp_or_mg && resto == 0 {
        1
    } else {
        resto
    }
}

fn titulo_dv1(seq: &[u32], is_sp_or_mg: bool) -> u32 {
    let sum: u32 = seq.iter().zip(TITULO_DV1_WEIGHTS).map(|(d, w)| d * w).sum();
    titulo_resto_to_dv(sum % 11, is_sp_or_mg)
}

fn titulo_dv2(uf: &[u32], dv1: u32, is_sp_or_mg: bool) -> u32 {
    let sum = uf[0] * 7 + uf[1] * 8 + dv1 * 9;
    titulo_resto_to_dv(sum % 11, is_sp_or_mg)
}

/// Título de eleitor (12 digits: 8-digit sequential + 2-digit UF code +
/// 2 chained check digits, research note 0001 §3.5).
fn validate_titulo_eleitor(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 12 {
        return false;
    }
    let seq = &ds[..8];
    let uf = &ds[8..10];
    let uf_value = uf[0] * 10 + uf[1];
    if !(1..=28).contains(&uf_value) {
        return false;
    }
    let is_sp_or_mg = uf_value == 1 || uf_value == 2;
    let dv1 = titulo_dv1(seq, is_sp_or_mg);
    let dv2 = titulo_dv2(uf, dv1, is_sp_or_mg);
    dv1 == ds[10] && dv2 == ds[11]
}

const CNH_DV1_WEIGHTS: [u32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, 1];
const CNH_DV2_WEIGHTS: [u32; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

fn cnh_dv1_and_correction(base: &[u32]) -> (u32, i32) {
    let sum: u32 = base.iter().zip(CNH_DV1_WEIGHTS).map(|(d, w)| d * w).sum();
    let resto = sum % 11;
    if resto >= 10 {
        (0, 2)
    } else {
        (resto, 0)
    }
}

/// DENATRAN's second CNH check digit subtracts a correction factor carried
/// over from the first pass (`x = 2` only when DV1 overflowed to 0); a
/// negative result wraps mod 11 rather than clamping to 0 (research note
/// 0001 §3.6).
fn cnh_dv2(base: &[u32], x: i32) -> u32 {
    let sum: u32 = base.iter().zip(CNH_DV2_WEIGHTS).map(|(d, w)| d * w).sum();
    let resto = sum % 11;
    let raw_dv2 = if resto >= 10 { 0 } else { resto as i32 };
    let corrected = raw_dv2 - x;
    (if corrected < 0 {
        corrected + 11
    } else {
        corrected
    }) as u32
}

/// CNH (11 digits, two chained check digits, research note 0001 §3.6).
fn validate_cnh(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    let base = &ds[..9];
    let (dv1, x) = cnh_dv1_and_correction(base);
    let dv2 = cnh_dv2(base, x);
    dv1 == ds[9] && dv2 == ds[10]
}

/// The definitive CNS shape (research note 0001 §3.7) constrains only the
/// first 14 digits (regime prefix `1`/`2` plus a fixed `00[01]` tail); the
/// 15th digit is the check digit and is verified by the unified sum below,
/// not by shape.
fn cns_shape_is_valid(ds: &[u32]) -> bool {
    if ds.len() != 15 {
        return false;
    }
    let definitive =
        matches!(ds[0], 1 | 2) && ds[11] == 0 && ds[12] == 0 && matches!(ds[13], 0..=1);
    let provisional = matches!(ds[0], 7..=9);
    definitive || provisional
}

/// CNS/Cartão SUS (15 digits, dual regime: definitive prefix 1/2 or
/// provisional prefix 7/8/9, unified by a single mod-11 sum, research note
/// 0001 §3.7).
fn validate_cns(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if !cns_shape_is_valid(&ds) {
        return false;
    }
    let sum: u32 = ds
        .iter()
        .enumerate()
        .map(|(i, d)| d * (15 - i as u32))
        .sum();
    sum.is_multiple_of(11)
}

const RENAVAM_DV_WEIGHTS: [u32; 10] = [3, 2, 9, 8, 7, 6, 5, 4, 3, 2];

/// RENAVAM (11 digits, one check digit over a `(soma * 10) % 11` variant of
/// the mod-11 rule, research note 0001 §3.8).
fn validate_renavam(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    let base = &ds[..10];
    let sum: u32 = base
        .iter()
        .zip(RENAVAM_DV_WEIGHTS)
        .map(|(d, w)| d * w)
        .sum();
    let dv = (sum * 10) % 11;
    let dv = if dv == 10 { 0 } else { dv };
    dv == ds[10]
}

pub(super) fn detectors() -> Vec<super::PiiDetector> {
    vec![
        super::PiiDetector {
            class: super::PiiClass::Cpf,
            pattern: Regex::new(r"\b\d{3}\.?\d{3}\.?\d{3}-?\d{2}\b").expect("valid cpf regex"),
            validate: validate_cpf,
        },
        super::PiiDetector {
            class: super::PiiClass::Cnpj,
            pattern: Regex::new(r"\b[0-9A-Z]{2}\.?[0-9A-Z]{3}\.?[0-9A-Z]{3}/?[0-9A-Z]{4}-?\d{2}\b")
                .expect("valid cnpj regex"),
            validate: validate_cnpj,
        },
        super::PiiDetector {
            class: super::PiiClass::Pis,
            pattern: Regex::new(r"\b\d{3}\.?\d{5}\.?\d{2}-?\d{1}\b").expect("valid pis regex"),
            validate: validate_pis,
        },
        super::PiiDetector {
            class: super::PiiClass::TituloEleitor,
            pattern: Regex::new(r"\b\d{12}\b").expect("valid titulo eleitor regex"),
            validate: validate_titulo_eleitor,
        },
        super::PiiDetector {
            class: super::PiiClass::Cnh,
            pattern: Regex::new(r"\b\d{11}\b").expect("valid cnh regex"),
            validate: validate_cnh,
        },
        super::PiiDetector {
            class: super::PiiClass::Cns,
            pattern: Regex::new(r"\b[1-9]\d{14}\b").expect("valid cns regex"),
            validate: validate_cns,
        },
        super::PiiDetector {
            class: super::PiiClass::Renavam,
            pattern: Regex::new(r"\b\d{11}\b").expect("valid renavam regex"),
            validate: validate_renavam,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_cpf_accepts_a_checksum_valid_formatted_cpf() {
        assert!(validate_cpf("111.444.777-35"));
    }

    #[test]
    fn validate_cpf_accepts_a_checksum_valid_bare_cpf() {
        assert!(validate_cpf("11144477735"));
    }

    #[test]
    fn validate_cpf_rejects_a_broken_check_digit() {
        assert!(!validate_cpf("111.444.777-00"));
    }

    #[test]
    fn validate_cpf_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_cpf("111.111.111-11"));
    }

    #[test]
    fn validate_cnpj_accepts_a_checksum_valid_numeric_cnpj() {
        assert!(validate_cnpj("59.541.264/0001-03"));
    }

    #[test]
    fn validate_cnpj_accepts_a_checksum_valid_alphanumeric_cnpj() {
        assert!(validate_cnpj("12.ABC.345/01DE-35"));
    }

    #[test]
    fn validate_cnpj_rejects_a_broken_check_digit() {
        assert!(!validate_cnpj("59.541.264/0001-00"));
    }

    #[test]
    fn validate_cnpj_rejects_an_all_equal_base() {
        assert!(!validate_cnpj("11.111.111/1111-00"));
    }

    #[test]
    fn validate_pis_accepts_a_checksum_valid_pis() {
        assert!(validate_pis("120.06307.23-3"));
    }

    #[test]
    fn validate_pis_rejects_a_broken_check_digit() {
        assert!(!validate_pis("120.06307.23-0"));
    }

    #[test]
    fn validate_titulo_eleitor_accepts_a_checksum_valid_title_using_the_research_worked_example() {
        assert!(validate_titulo_eleitor("102385010671"));
    }

    #[test]
    fn validate_titulo_eleitor_accepts_the_sp_special_resto_zero_rule() {
        assert!(validate_titulo_eleitor("111111110116"));
    }

    #[test]
    fn validate_titulo_eleitor_accepts_the_mg_special_resto_zero_rule() {
        assert!(validate_titulo_eleitor("111111110213"));
    }

    #[test]
    fn validate_titulo_eleitor_rejects_a_broken_check_digit() {
        assert!(!validate_titulo_eleitor("102385010670"));
    }

    #[test]
    fn validate_titulo_eleitor_rejects_an_invalid_uf_code() {
        assert!(!validate_titulo_eleitor("102385012999"));
    }

    #[test]
    fn validate_cnh_accepts_a_checksum_valid_cnh() {
        assert!(validate_cnh("12345678900"));
    }

    #[test]
    fn validate_cnh_accepts_a_checksum_valid_cnh_through_the_x_correction_branch() {
        assert!(validate_cnh("98765432109"));
    }

    #[test]
    fn validate_cnh_rejects_a_broken_check_digit() {
        assert!(!validate_cnh("12345678901"));
    }

    #[test]
    fn validate_cnh_rejects_an_all_equal_digit_string() {
        assert!(!validate_cnh("11111111111"));
    }

    #[test]
    fn validate_cns_accepts_a_checksum_valid_definitive_card() {
        assert!(validate_cns("100000000000007"));
    }

    #[test]
    fn validate_cns_accepts_a_checksum_valid_provisional_card() {
        assert!(validate_cns("700000000000005"));
    }

    #[test]
    fn validate_cns_rejects_a_broken_check_digit() {
        assert!(!validate_cns("100000000000000"));
    }

    #[test]
    fn validate_renavam_accepts_a_checksum_valid_renavam() {
        assert!(validate_renavam("12345678900"));
    }

    #[test]
    fn validate_renavam_rejects_a_broken_check_digit() {
        assert!(!validate_renavam("12345678901"));
    }

    #[test]
    fn validate_renavam_rejects_an_all_equal_digit_string() {
        assert!(!validate_renavam("11111111111"));
    }

    /// Base `1834567890` sums to `243` over `RENAVAM_DV_WEIGHTS`, and
    /// `243 % 11 == 1`, so `(soma * 10) % 11 == 10` — exercising the
    /// `dv == 10 -> 0` remap branch a mutant deleting it would otherwise
    /// leave uncovered.
    #[test]
    fn validate_renavam_accepts_a_checksum_valid_renavam_through_the_ten_to_zero_remap() {
        assert!(validate_renavam("18345678900"));
    }

    #[test]
    fn detectors_registers_one_detector_per_document_class() {
        assert_eq!(detectors().len(), 7);
    }
}
