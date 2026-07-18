//! Global financial Tier 1 detectors (ADR 0012, research note 0001 §4):
//! credit card (issuer prefix + Luhn), IBAN (mod-97), US ABA routing number
//! (weighted 3-7-1 mod-10), and US NPI (Luhn over an `80840`-prefixed
//! digit string). Each pairs a permissive regex with a Rust validator, same
//! two-stage shape as `pii::brazil`.

use super::checksum;
use regex::Regex;

fn has_visa_prefix_and_len(ds: &[u32]) -> bool {
    matches!(ds.len(), 13 | 16 | 19) && ds[0] == 4
}

fn has_mastercard_prefix_and_len(ds: &[u32]) -> bool {
    if ds.len() != 16 {
        return false;
    }
    let two = ds[0] * 10 + ds[1];
    let four = ds[0] * 1000 + ds[1] * 100 + ds[2] * 10 + ds[3];
    (51..=55).contains(&two) || (2221..=2720).contains(&four)
}

fn has_amex_prefix_and_len(ds: &[u32]) -> bool {
    if ds.len() != 15 {
        return false;
    }
    let two = ds[0] * 10 + ds[1];
    two == 34 || two == 37
}

fn has_discover_prefix_and_len(ds: &[u32]) -> bool {
    if ds.len() != 16 {
        return false;
    }
    let two = ds[0] * 10 + ds[1];
    let three = two * 10 + ds[2];
    let four = three * 10 + ds[3];
    four == 6011 || two == 65 || (644..=649).contains(&three)
}

fn has_recognized_issuer(ds: &[u32]) -> bool {
    has_visa_prefix_and_len(ds)
        || has_mastercard_prefix_and_len(ds)
        || has_amex_prefix_and_len(ds)
        || has_discover_prefix_and_len(ds)
}

/// Payment card (13-19 digits, research note 0001 §2/§4): rejects an
/// all-equal placeholder, requires the digits to match a known issuer
/// scheme's prefix and length, then verifies Luhn over every digit. A
/// Luhn-valid number with no recognized issuer is not reported — matching
/// arbitrary 13-19 digit numbers would swamp the signal.
fn validate_credit_card(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if checksum::all_same(&ds) {
        return false;
    }
    has_recognized_issuer(&ds) && checksum::luhn_valid(&ds)
}

fn iban_char_value(c: char) -> Option<u32> {
    if let Some(d) = c.to_digit(10) {
        return Some(d);
    }
    if c.is_ascii_uppercase() {
        return Some(c as u32 - 'A' as u32 + 10);
    }
    None
}

fn iban_mod97_remainder(rearranged: &str) -> Option<u32> {
    let mut rem = 0u32;
    for c in rearranged.chars() {
        let value = iban_char_value(c)?;
        rem = if value >= 10 {
            (rem * 100 + value) % 97
        } else {
            (rem * 10 + value) % 97
        };
    }
    Some(rem)
}

/// IBAN (research note 0001 §4, ISO 13616 mod-97): strips to `A-Z0-9`,
/// moves the first 4 characters to the end, then folds the mod-97 checksum
/// digit-by-digit (two positions per letter, one per digit, since letters
/// expand to two-digit values `10..35`). Valid iff the final remainder is 1.
fn validate_iban(matched: &str) -> bool {
    let stripped: String = matched
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if !(15..=34).contains(&stripped.chars().count()) {
        return false;
    }
    let chars: Vec<char> = stripped.chars().collect();
    if chars.len() < 4 {
        return false;
    }
    let rearranged: String = chars[4..].iter().chain(chars[..4].iter()).collect();
    matches!(iban_mod97_remainder(&rearranged), Some(1))
}

const ABA_WEIGHTS: [u32; 9] = [3, 7, 1, 3, 7, 1, 3, 7, 1];

/// US ABA routing number (9 digits, research note 0001 §4): rejects an
/// all-equal placeholder, then requires the weighted `3-7-1` sum to be a
/// non-zero multiple of 10 (a zero sum only arises from an all-zero input,
/// which `all_same` already rejects, but the guard documents the invariant).
fn validate_aba_routing(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    let sum: u32 = ds.iter().zip(ABA_WEIGHTS).map(|(d, w)| d * w).sum();
    sum != 0 && sum.is_multiple_of(10)
}

/// US NPI (10 digits, research note 0001 §4): rejects an all-equal
/// placeholder, then runs Luhn over the ISO issuer prefix `80840` followed
/// by the 10 NPI digits — folding the prefix into the checksum is how the
/// NPI standard disambiguates a plain 10-digit Luhn pass from a real NPI.
fn validate_npi(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 10 || checksum::all_same(&ds) {
        return false;
    }
    let mut prefixed = checksum::digits("80840");
    prefixed.extend(ds);
    checksum::luhn_valid(&prefixed)
}

pub(super) fn detectors() -> Vec<super::PiiDetector> {
    vec![
        super::PiiDetector {
            label: "Credit card number",
            pattern: Regex::new(r"\b\d{13,19}\b").expect("valid credit card regex"),
            validate: validate_credit_card,
        },
        super::PiiDetector {
            label: "IBAN",
            pattern: Regex::new(r"\b[A-Z]{2}\d{2}[A-Z0-9]{9,30}\b").expect("valid iban regex"),
            validate: validate_iban,
        },
        super::PiiDetector {
            label: "US ABA routing number",
            pattern: Regex::new(r"\b\d{9}\b").expect("valid aba routing regex"),
            validate: validate_aba_routing,
        },
        super::PiiDetector {
            label: "US NPI",
            pattern: Regex::new(r"\b\d{10}\b").expect("valid npi regex"),
            validate: validate_npi,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_credit_card_accepts_a_checksum_valid_visa() {
        assert!(validate_credit_card("4111111111111111"));
    }

    #[test]
    fn validate_credit_card_accepts_a_checksum_valid_mastercard() {
        assert!(validate_credit_card("5555555555554444"));
    }

    #[test]
    fn validate_credit_card_accepts_a_checksum_valid_amex() {
        assert!(validate_credit_card("378282246310005"));
    }

    #[test]
    fn validate_credit_card_rejects_a_luhn_invalid_number() {
        assert!(!validate_credit_card("4111111111111112"));
    }

    #[test]
    fn validate_credit_card_rejects_a_luhn_valid_number_with_no_recognized_issuer() {
        assert!(!validate_credit_card("1234567890123"));
    }

    #[test]
    fn validate_credit_card_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_credit_card("4444444444444444"));
    }

    #[test]
    fn validate_iban_accepts_a_checksum_valid_gb_iban() {
        assert!(validate_iban("GB82WEST12345698765432"));
    }

    #[test]
    fn validate_iban_accepts_a_checksum_valid_de_iban() {
        assert!(validate_iban("DE89370400440532013000"));
    }

    #[test]
    fn validate_iban_rejects_a_broken_check_digits() {
        assert!(!validate_iban("GB82WEST12345698765433"));
    }

    #[test]
    fn validate_aba_routing_accepts_a_checksum_valid_routing_number() {
        assert!(validate_aba_routing("021000021"));
    }

    #[test]
    fn validate_aba_routing_rejects_a_broken_checksum() {
        assert!(!validate_aba_routing("021000022"));
    }

    #[test]
    fn validate_aba_routing_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_aba_routing("111111111"));
    }

    #[test]
    fn validate_npi_accepts_a_checksum_valid_npi() {
        assert!(validate_npi("1234567893"));
    }

    #[test]
    fn validate_npi_rejects_a_broken_check_digit() {
        assert!(!validate_npi("1234567890"));
    }

    #[test]
    fn validate_npi_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_npi("1111111111"));
    }

    #[test]
    fn detectors_registers_one_detector_per_financial_class() {
        assert_eq!(detectors().len(), 4);
    }
}
