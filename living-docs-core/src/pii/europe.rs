//! Tier 1 EU national-ID detectors (ADR 0012, research note 0001 §2/§4):
//! Spain NIF/NIE (mod-23 check letter), Portugal NIF (weighted mod-11),
//! Netherlands BSN (11-proef), Italy Codice Fiscale (odd/even table
//! mod-26 check letter), and Germany Steuer-IdNr (ISO 7064 MOD 11,10).
//! Same two-stage regex+validator shape as `pii::brazil` and
//! `pii::financial`.

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

/// The Italy Codice Fiscale odd-position table (research note 0001 §4),
/// indexed `0..=9` for digits `'0'..='9'` and `10..=35` for letters
/// `'A'..='Z'`: unlike the even-position table (plain digit/ordinal value),
/// this mapping has no closed-form formula, so it is data, not a branch.
const ITALY_ODD_VALUES: [u32; 36] = [
    1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 1, 0, 5, 7, 9, 13, 15, 17, 19, 21, 2, 4, 18, 20, 11, 3, 6,
    8, 12, 14, 16, 10, 22, 25, 24, 23,
];

fn italy_odd_table_index(c: char) -> Option<usize> {
    if let Some(digit) = c.to_digit(10) {
        return Some(digit as usize);
    }
    if c.is_ascii_uppercase() {
        return Some(10 + (c as u8 - b'A') as usize);
    }
    None
}

fn italy_even_table_value(c: char) -> Option<u32> {
    if let Some(digit) = c.to_digit(10) {
        return Some(digit);
    }
    if c.is_ascii_uppercase() {
        return Some((c as u8 - b'A') as u32);
    }
    None
}

/// Sums the ODD-table value at each 1-indexed odd position (0-indexed even
/// index) and the EVEN-table value at each 1-indexed even position over the
/// first 15 Codice Fiscale characters, then folds the sum to the mod-26
/// check letter (research note 0001 §4).
fn italy_check_letter(chars: &[char]) -> Option<char> {
    let mut sum = 0u32;
    for (i, &c) in chars.iter().enumerate() {
        let value = if i % 2 == 0 {
            ITALY_ODD_VALUES[italy_odd_table_index(c)?]
        } else {
            italy_even_table_value(c)?
        };
        sum += value;
    }
    Some(char::from(b'A' + (sum % 26) as u8))
}

/// Italy Codice Fiscale (16 chars, research note 0001 §2/§4): requires 16
/// uppercase alnum characters, then the 16th must equal the mod-26 check
/// letter computed over the first 15.
fn validate_italy_codice_fiscale(matched: &str) -> bool {
    let chars: Vec<char> = matched.chars().collect();
    if chars.len() != 16
        || !chars
            .iter()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return false;
    }
    let Some(check) = italy_check_letter(&chars[..15]) else {
        return false;
    };
    check == chars[15]
}

/// Germany Steuer-IdNr (11 digits, research note 0001 §2/§4): rejects an
/// all-equal placeholder, then runs the ISO 7064 MOD 11,10 loop over the
/// first 10 digits and requires the derived check digit to equal the 11th.
fn validate_germany_steuer_id(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    let product = ds[..10].iter().fold(10u32, |product, &digit| {
        let mut m = (digit + product) % 10;
        if m == 0 {
            m = 10;
        }
        (m * 2) % 11
    });
    let check = (11 - product) % 10;
    check == ds[10]
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
        super::PiiDetector {
            label: "Italian Codice Fiscale",
            pattern: Regex::new(r"\b[A-Z0-9]{16}\b").expect("valid italy codice fiscale regex"),
            validate: validate_italy_codice_fiscale,
        },
        super::PiiDetector {
            label: "German Steuer-IdNr",
            pattern: Regex::new(r"\b\d{11}\b").expect("valid germany steuer-idnr regex"),
            validate: validate_germany_steuer_id,
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
    fn validate_italy_codice_fiscale_accepts_the_canonical_valid_vector() {
        assert!(validate_italy_codice_fiscale("RSSMRA85T10A562S"));
    }

    #[test]
    fn validate_italy_codice_fiscale_rejects_a_wrong_check_letter() {
        assert!(!validate_italy_codice_fiscale("RSSMRA85T10A562A"));
    }

    #[test]
    fn validate_italy_codice_fiscale_rejects_a_string_shorter_than_16_chars() {
        assert!(!validate_italy_codice_fiscale("RSSMRA85T10A562"));
    }

    #[test]
    fn validate_italy_codice_fiscale_rejects_lowercase_input() {
        assert!(!validate_italy_codice_fiscale("rssmra85t10a562s"));
    }

    #[test]
    fn validate_germany_steuer_id_accepts_the_canonical_valid_vector() {
        assert!(validate_germany_steuer_id("86095742719"));
    }

    #[test]
    fn validate_germany_steuer_id_rejects_a_broken_check_digit() {
        assert!(!validate_germany_steuer_id("86095742718"));
    }

    #[test]
    fn validate_germany_steuer_id_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_germany_steuer_id("11111111111"));
    }

    #[test]
    fn validate_germany_steuer_id_accepts_a_vector_exercising_the_iso7064_correction_branch() {
        assert!(validate_germany_steuer_id("02345678910"));
    }

    #[test]
    fn detectors_registers_one_detector_per_europe_class() {
        assert_eq!(detectors().len(), 5);
    }
}
