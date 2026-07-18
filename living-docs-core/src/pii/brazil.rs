//! Brazil Tier 1 detectors (ADR 0012, research note 0001 §3.1-3.4): CPF,
//! CNPJ (numeric and the 2026 alphanumeric form via a single detector), and
//! PIS/PASEP/NIT. Each pairs a permissive regex with a Rust validator that
//! normalizes separators, rejects trivial (all-equal-digit) inputs, and
//! verifies the mod-11 check digit(s).

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
    fn detectors_registers_one_detector_per_document_class() {
        assert_eq!(detectors().len(), 3);
    }
}
