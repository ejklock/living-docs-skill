//! Tier 1 EU national-ID detectors, part 1 (ADR 0012, research note 0001
//! §2/§4): Spain NIF/NIE (mod-23 check letter), Portugal NIF (weighted
//! mod-11), and Netherlands BSN (11-proef). Same two-stage regex+validator
//! shape as `pii::brazil` and `pii::financial`.

use super::checksum;
use regex::Regex;

const SPAIN_CHECK_LETTERS: &str = "TRWAGMYFPDXBNJZSQVHLCKE";

fn spain_check_letter(number: u32) -> char {
    let idx = (number % 23) as usize;
    SPAIN_CHECK_LETTERS
        .chars()
        .nth(idx)
        .expect("index is always within the 23-letter table")
}

/// Splits a NIE's leading `X`/`Y`/`Z` from the trailing digits, if present;
/// a plain NIF has no prefix and every remaining character is a digit.
fn split_spain_prefix(rest: &[char]) -> (Option<char>, &[char]) {
    match rest.split_first() {
        Some((&c, tail)) if matches!(c, 'X' | 'Y' | 'Z') => (Some(c), tail),
        _ => (None, rest),
    }
}

fn spain_prefix_value(prefix: char) -> u32 {
    match prefix {
        'X' => 0,
        'Y' => 1,
        _ => 2,
    }
}

/// Parses a Spain NIF/NIE candidate into the check number and its trailing
/// letter: a NIE's `X`/`Y`/`Z` prefix is mapped to `0`/`1`/`2` and folded
/// into the number as its leading digit (research note 0001 §2) so NIF and
/// NIE share one mod-23 lookup.
fn spain_number_and_letter(matched: &str) -> Option<(u32, char)> {
    let chars: Vec<char> = matched.chars().collect();
    let (&letter, rest) = chars.split_last()?;
    if !letter.is_ascii_uppercase() {
        return None;
    }
    let (prefix, digit_chars) = split_spain_prefix(rest);
    let expected_len = if prefix.is_some() { 7 } else { 8 };
    if digit_chars.len() != expected_len || !digit_chars.iter().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let digits_value = digit_chars
        .iter()
        .fold(0u32, |acc, c| acc * 10 + c.to_digit(10).unwrap_or(0));
    let prefix_value = prefix.map(spain_prefix_value).unwrap_or(0);
    Some((prefix_value * 10_000_000 + digits_value, letter))
}

/// Spain NIF/NIE (research note 0001 §2, mod-23 check letter): a NIF is 8
/// digits, a NIE is `X`/`Y`/`Z` + 7 digits; either way the trailing letter
/// must equal `SPAIN_CHECK_LETTERS[number % 23]`.
fn validate_spain_nif_nie(matched: &str) -> bool {
    let Some((number, letter)) = spain_number_and_letter(matched) else {
        return false;
    };
    spain_check_letter(number) == letter
}

const PORTUGAL_NIF_WEIGHTS: [u32; 8] = [9, 8, 7, 6, 5, 4, 3, 2];

/// Portugal NIF (9 digits, research note 0001 §2): rejects an all-equal
/// placeholder, then requires the 9th digit to equal the weighted mod-11
/// check digit over the first 8 (shared `checksum::weighted_mod11_dv` rule:
/// `11 - resto`, folded to `0` when that would be `10` or `11`).
fn validate_portugal_nif(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    let check = checksum::weighted_mod11_dv(&ds[..8], &PORTUGAL_NIF_WEIGHTS);
    check == ds[8]
}

const NETHERLANDS_BSN_WEIGHTS: [i32; 9] = [9, 8, 7, 6, 5, 4, 3, 2, -1];

/// Netherlands BSN (9 digits, research note 0001 §2, "11-proef"): rejects an
/// all-equal (including all-zero) placeholder, then requires the weighted
/// sum — whose last weight is `-1`, unlike every other mod-11 detector in
/// this crate — to be a multiple of 11.
fn validate_netherlands_bsn(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    let sum: i32 = ds
        .iter()
        .zip(NETHERLANDS_BSN_WEIGHTS)
        .map(|(d, w)| *d as i32 * w)
        .sum();
    sum % 11 == 0
}

pub(super) fn detectors() -> Vec<super::PiiDetector> {
    vec![
        super::PiiDetector {
            label: "Spanish NIF/NIE",
            pattern: Regex::new(r"\b[XYZ]?\d{7,8}[A-Z]\b").expect("valid spain nif/nie regex"),
            validate: validate_spain_nif_nie,
        },
        super::PiiDetector {
            label: "Portuguese NIF",
            pattern: Regex::new(r"\b\d{9}\b").expect("valid portugal nif regex"),
            validate: validate_portugal_nif,
        },
        super::PiiDetector {
            label: "Dutch BSN",
            pattern: Regex::new(r"\b\d{9}\b").expect("valid netherlands bsn regex"),
            validate: validate_netherlands_bsn,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_spain_nif_nie_accepts_a_checksum_valid_nif() {
        assert!(validate_spain_nif_nie("12345678Z"));
    }

    #[test]
    fn validate_spain_nif_nie_accepts_a_checksum_valid_nie() {
        assert!(validate_spain_nif_nie("X1234567L"));
    }

    #[test]
    fn validate_spain_nif_nie_accepts_a_valid_y_prefixed_nie() {
        assert!(validate_spain_nif_nie("Y1234567X"));
    }

    #[test]
    fn validate_spain_nif_nie_accepts_a_valid_z_prefixed_nie() {
        assert!(validate_spain_nif_nie("Z1234567R"));
    }

    #[test]
    fn validate_spain_nif_nie_rejects_a_wrong_check_letter() {
        assert!(!validate_spain_nif_nie("12345678A"));
    }

    #[test]
    fn validate_spain_nif_nie_rejects_a_nie_with_a_wrong_check_letter() {
        assert!(!validate_spain_nif_nie("X1234567A"));
    }

    #[test]
    fn validate_portugal_nif_accepts_a_checksum_valid_nif() {
        assert!(validate_portugal_nif("123456789"));
    }

    #[test]
    fn validate_portugal_nif_rejects_a_broken_check_digit() {
        assert!(!validate_portugal_nif("123456780"));
    }

    #[test]
    fn validate_portugal_nif_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_portugal_nif("111111111"));
    }

    #[test]
    fn validate_netherlands_bsn_accepts_a_checksum_valid_bsn() {
        assert!(validate_netherlands_bsn("111222333"));
    }

    #[test]
    fn validate_netherlands_bsn_rejects_a_broken_checksum() {
        assert!(!validate_netherlands_bsn("111222334"));
    }

    #[test]
    fn validate_netherlands_bsn_rejects_an_all_zero_placeholder() {
        assert!(!validate_netherlands_bsn("000000000"));
    }

    #[test]
    fn validate_netherlands_bsn_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_netherlands_bsn("222222222"));
    }

    #[test]
    fn detectors_registers_one_detector_per_europe_class() {
        assert_eq!(detectors().len(), 3);
    }
}
