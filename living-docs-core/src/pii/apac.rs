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

/// South Korea Resident Registration Number (13 digits, research note 0001):
/// the regex already pins digit 7 (the gender/century flag) to `[1-4]`, so
/// the validator does not re-check it — a second guard over territory the
/// regex already owns would be dead code (B8a South Africa citizenship-flag
/// lesson). After rejecting an all-equal placeholder, the check digit is the
/// weighted mod-11 residue over the first 12 digits, rescaled into `0..=9`.
fn validate_korea_rrn(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 13 || checksum::all_same(&ds) {
        return false;
    }
    const WEIGHTS: [u32; 12] = [2, 3, 4, 5, 6, 7, 8, 9, 2, 3, 4, 5];
    let sum: u32 = ds[..12].iter().zip(WEIGHTS).map(|(d, w)| d * w).sum();
    let check = (11 - (sum % 11)) % 10;
    check == ds[12]
}

/// Australia Tax File Number (9 digits, research note 0001): after rejecting
/// an all-equal placeholder, valid iff the digit-weighted sum is an exact
/// multiple of 11.
fn validate_australia_tfn(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    const WEIGHTS: [u32; 9] = [1, 4, 3, 7, 5, 8, 6, 9, 10];
    let sum: u32 = ds.iter().zip(WEIGHTS).map(|(d, w)| d * w).sum();
    sum.is_multiple_of(11)
}

/// Australia Business Number (11 digits, research note 0001): the leading
/// digit is decremented by 1 before weighting, per the published ABN
/// algorithm — guarded so a leading `0` (which would underflow the
/// decrement) is rejected outright rather than validated against the wrong
/// sequence. Valid iff the adjusted digit-weighted sum is an exact multiple
/// of 89.
fn validate_australia_abn(matched: &str) -> bool {
    let mut ds = checksum::digits(matched);
    if ds.len() != 11 || checksum::all_same(&ds) {
        return false;
    }
    if ds[0] < 1 {
        return false;
    }
    ds[0] -= 1;
    const WEIGHTS: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
    let sum: u32 = ds.iter().zip(WEIGHTS).map(|(d, w)| d * w).sum();
    sum.is_multiple_of(89)
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
        super::PiiDetector {
            label: "Korean RRN",
            pattern: Regex::new(r"\b\d{6}-?[1-4]\d{6}\b").expect("valid korea rrn regex"),
            validate: validate_korea_rrn,
        },
        super::PiiDetector {
            label: "Australian TFN",
            pattern: Regex::new(r"\b\d{9}\b").expect("valid australia tfn regex"),
            validate: validate_australia_tfn,
        },
        super::PiiDetector {
            label: "Australian ABN",
            pattern: Regex::new(r"\b\d{11}\b").expect("valid australia abn regex"),
            validate: validate_australia_abn,
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
    fn validate_korea_rrn_accepts_the_derived_checksum_valid_vector() {
        assert!(validate_korea_rrn("9701011234569"));
    }

    #[test]
    fn validate_korea_rrn_rejects_a_broken_check_digit() {
        assert!(!validate_korea_rrn("9701011234568"));
    }

    #[test]
    fn validate_korea_rrn_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_korea_rrn("1111111111111"));
    }

    #[test]
    fn validate_australia_tfn_accepts_the_canonical_valid_vector() {
        assert!(validate_australia_tfn("123456782"));
    }

    #[test]
    fn validate_australia_tfn_rejects_a_broken_checksum() {
        assert!(!validate_australia_tfn("123456780"));
    }

    #[test]
    fn validate_australia_tfn_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_australia_tfn("999999999"));
    }

    #[test]
    fn validate_australia_abn_accepts_the_canonical_valid_vector() {
        assert!(validate_australia_abn("51824753556"));
    }

    #[test]
    fn validate_australia_abn_rejects_a_broken_checksum() {
        assert!(!validate_australia_abn("51824753557"));
    }

    #[test]
    fn validate_australia_abn_rejects_an_all_equal_digit_placeholder() {
        assert!(!validate_australia_abn("77777777777"));
    }

    /// `51824753556` is the canonical valid ABN vector, but only once the
    /// leading digit is decremented per the published algorithm: without
    /// that adjustment the weighted sum is `544`, not a multiple of `89`
    /// (`534` is `6 * 89`; `544` is not) — proving the `-1` step is load
    /// bearing, not incidental.
    #[test]
    fn validate_australia_abn_rejects_the_canonical_vector_when_the_leading_digit_adjustment_is_skipped(
    ) {
        let ds = checksum::digits("51824753556");
        const WEIGHTS: [u32; 11] = [10, 1, 3, 5, 7, 9, 11, 13, 15, 17, 19];
        let unadjusted_sum: u32 = ds.iter().zip(WEIGHTS).map(|(d, w)| d * w).sum();
        assert!(!unadjusted_sum.is_multiple_of(89));
    }

    #[test]
    fn detectors_registers_one_detector_per_apac_class() {
        assert_eq!(detectors().len(), 5);
    }
}
