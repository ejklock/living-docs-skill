//! APAC/Africa Tier 1 detectors (ADR 0012, research note 0001): India
//! Aadhaar (Verhoeff check digit over 12 digits) and South Africa National
//! ID (embedded birth date + Luhn over 13 digits). Same two-stage
//! regex+validator shape as `pii::brazil` and `pii::europe`.

use super::checksum;
use regex::Regex;

/// True when `values` reads the same forwards and backwards — Aadhaar
/// issuance avoids these as a human-guessable placeholder pattern, the same
/// way `checksum::all_same` rejects an all-repeated-digit placeholder.
fn is_digit_palindrome(values: &[u32]) -> bool {
    values.iter().eq(values.iter().rev())
}

/// India Aadhaar (12 digits, research note 0001): rejects an all-equal or
/// palindromic placeholder and a leading digit below `2` (Aadhaar never
/// starts with `0`/`1`), then defers to the shared `checksum::verhoeff_valid`
/// over the full 12-digit sequence.
///
/// The Aadhaar valid test vector `234567890124` is derived, not invented: the
/// 11-digit base `23456789012` (first digit `2`, not a palindrome) fed to the
/// Verhoeff check-digit derivation yields `4`, and appending it makes
/// `checksum::verhoeff_valid` return `true` (`checksum` module tests pin the
/// same derivation against the externally-documented `236` -> `3` example).
fn validate_india_aadhaar(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 12 || checksum::all_same(&ds) || is_digit_palindrome(&ds) {
        return false;
    }
    if ds[0] < 2 {
        return false;
    }
    checksum::verhoeff_valid(&ds)
}

fn south_africa_date_in_range(ds: &[u32]) -> bool {
    let month = ds[2] * 10 + ds[3];
    let day = ds[4] * 10 + ds[5];
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// South Africa National ID (13 digits, research note 0001): the leading 6
/// digits are a `YYMMDD` birth date, digit 10 is the citizenship flag
/// (`0`/`1` citizen, `2` permanent resident), and the whole 13-digit sequence
/// is a Luhn checksum. An out-of-range month/day is rejected before Luhn
/// runs, since Luhn alone cannot distinguish a bad date from a bad sequence
/// number (mirrors `validate_sweden_personnummer`'s date-then-Luhn order).
fn validate_south_africa_id(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 13 {
        return false;
    }
    if !south_africa_date_in_range(&ds) {
        return false;
    }
    if !(0..=2).contains(&ds[10]) {
        return false;
    }
    checksum::luhn_valid(&ds)
}

pub(super) fn detectors() -> Vec<super::PiiDetector> {
    vec![
        super::PiiDetector {
            label: "Indian Aadhaar",
            pattern: Regex::new(r"\b[2-9]\d{3}[- ]?\d{4}[- ]?\d{4}\b")
                .expect("valid india aadhaar regex"),
            validate: validate_india_aadhaar,
        },
        super::PiiDetector {
            label: "South African ID",
            pattern: Regex::new(r"\b\d{10}[0-2][89]\d\b").expect("valid south africa id regex"),
            validate: validate_south_africa_id,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_india_aadhaar_accepts_the_derived_checksum_valid_vector() {
        assert!(validate_india_aadhaar("234567890124"));
    }

    #[test]
    fn validate_india_aadhaar_accepts_a_separator_formatted_vector() {
        assert!(validate_india_aadhaar("2345 6789 0124"));
    }

    #[test]
    fn validate_india_aadhaar_rejects_a_broken_verhoeff_check_digit() {
        assert!(!validate_india_aadhaar("234567890120"));
    }

    /// Base `01234567890` (leading digit `0`) carries the Verhoeff check
    /// digit `6`, so `012345678906` passes `checksum::verhoeff_valid` outright
    /// — the leading-digit guard is the only reason this vector is rejected.
    #[test]
    fn validate_india_aadhaar_rejects_a_leading_digit_below_two_even_when_verhoeff_valid() {
        assert!(!validate_india_aadhaar("012345678906"));
    }

    /// `200009900002` reads the same forwards and backwards and its trailing
    /// `2` is exactly the Verhoeff check digit for base `20000990000`, so it
    /// passes `checksum::verhoeff_valid` outright — the palindrome guard is
    /// the only reason this vector is rejected.
    #[test]
    fn validate_india_aadhaar_rejects_a_digit_palindrome_even_when_verhoeff_valid() {
        assert!(!validate_india_aadhaar("200009900002"));
    }

    #[test]
    fn validate_india_aadhaar_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_india_aadhaar("222222222222"));
    }

    #[test]
    fn validate_south_africa_id_accepts_the_canonical_valid_vector() {
        assert!(validate_south_africa_id("8001015009087"));
    }

    #[test]
    fn validate_south_africa_id_rejects_a_broken_luhn_checksum() {
        assert!(!validate_south_africa_id("8001015009086"));
    }

    /// `8013010009087` swaps the canonical vector's month to `13`
    /// (impossible) while keeping day `01`; the trailing digit `7` is chosen
    /// so the full 13-digit sequence still passes `checksum::luhn_valid` —
    /// the date guard is the only reason this vector is rejected.
    #[test]
    fn validate_south_africa_id_rejects_an_impossible_month_even_when_luhn_valid() {
        assert!(!validate_south_africa_id("8013010009087"));
    }

    /// `8001015009384` swaps the canonical vector's citizenship digit to `3`
    /// (out of the `0..=2` range) with a trailing digit chosen so the full
    /// 13-digit sequence still passes `checksum::luhn_valid` — the
    /// citizenship guard is the only reason this vector is rejected.
    #[test]
    fn validate_south_africa_id_rejects_an_out_of_range_citizenship_digit_even_when_luhn_valid() {
        assert!(!validate_south_africa_id("8001015009384"));
    }

    #[test]
    fn detectors_registers_one_detector_per_apac_class() {
        assert_eq!(detectors().len(), 2);
    }
}
