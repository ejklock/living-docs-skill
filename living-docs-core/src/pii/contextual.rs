//! Tier-2 context-gated PII detectors. Tier-1 (`PiiDetector`) validates a
//! checksum, so the checksum itself is the false-positive filter and a
//! Tier-1 match is reported unconditionally. Some identifiers — starting
//! with the US SSN — have no checksum, only structural rules (digit ranges,
//! forbidden placeholders), and structure alone is a weak filter: plenty of
//! ordinary 9-digit numbers pass it. `ContextualDetector` adds `context`, a
//! set of nearby words required before a structurally-valid candidate is
//! reported, so a nearby label (e.g. "SSN") stands in for the missing
//! checksum as the false-positive filter. This is why Tier-2 is a distinct
//! type rather than an optional field on `PiiDetector`: Tier-1 detectors
//! must keep firing unconditionally, and folding an always-`None` `context`
//! onto every Tier-1 detector would let a caller accidentally gate one.

use super::checksum;
use regex::Regex;

/// A context-gated detector: `pattern` finds a syntactic candidate,
/// `validate` checks it structurally (no checksum), and `context` lists the
/// nearby words that must accompany a structurally-valid candidate before it
/// is reported.
pub(super) struct ContextualDetector {
    pub(super) label: &'static str,
    pub(super) pattern: Regex,
    pub(super) validate: fn(&str) -> bool,
    pub(super) context: &'static [&'static str],
}

fn ssn_area(ds: &[u32]) -> u32 {
    ds[0] * 100 + ds[1] * 10 + ds[2]
}

fn ssn_group(ds: &[u32]) -> u32 {
    ds[3] * 10 + ds[4]
}

fn ssn_serial(ds: &[u32]) -> u32 {
    ds[5] * 1000 + ds[6] * 100 + ds[7] * 10 + ds[8]
}

fn is_invalid_ssn_area(area: u32) -> bool {
    area == 0 || area == 666 || area >= 900
}

const SSN_PLACEHOLDERS: [[u32; 9]; 3] = [
    [0, 7, 8, 0, 5, 1, 1, 2, 0],
    [2, 1, 9, 0, 9, 9, 9, 9, 9],
    [1, 2, 3, 4, 5, 6, 7, 8, 9],
];

fn is_placeholder_ssn(ds: &[u32]) -> bool {
    SSN_PLACEHOLDERS.iter().any(|placeholder| placeholder == ds)
}

/// US SSN (9 digits, no checksum — SSA structural rules only, research note
/// 0001 §4): rejects an all-equal run, an area of `000`/`666`/`900-999`, a
/// zero group or zero serial, and the well-known placeholder sequences
/// (078-05-1120 shipped on sample SSN cards for decades; 219-09-9999 and
/// 123-45-6789 are common filler).
fn validate_us_ssn(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    if is_invalid_ssn_area(ssn_area(&ds)) {
        return false;
    }
    if ssn_group(&ds) == 0 || ssn_serial(&ds) == 0 {
        return false;
    }
    !is_placeholder_ssn(&ds)
}

/// Valid US ITIN group ranges (digits 3..5 of the 9-digit number), per IRS
/// assignment: `50-65`, `70-88`, `90-92`, `94-99`. The leading `9` is already
/// pinned by the detector's regex, so `validate_us_itin` never re-checks it
/// (B8a dead-guard lesson).
fn is_valid_itin_group(group: u32) -> bool {
    (50..=65).contains(&group)
        || (70..=88).contains(&group)
        || (90..=92).contains(&group)
        || (94..=99).contains(&group)
}

/// US ITIN (9 digits, no checksum — IRS structural rule only): rejects an
/// all-equal placeholder, then requires the group (digits 3..5) to fall in
/// one of the IRS-assigned ranges. The leading `9` is regex-owned.
fn validate_us_itin(matched: &str) -> bool {
    let ds = checksum::digits(matched);
    if ds.len() != 9 || checksum::all_same(&ds) {
        return false;
    }
    is_valid_itin_group(ds[3] * 10 + ds[4])
}

const NINO_INVALID_FIRST_LETTERS: [char; 6] = ['D', 'F', 'I', 'Q', 'U', 'V'];
const NINO_INVALID_SECOND_LETTERS: [char; 7] = ['D', 'F', 'I', 'O', 'Q', 'U', 'V'];
const NINO_INVALID_PREFIXES: [&str; 7] = ["BG", "GB", "NK", "KN", "NT", "TN", "ZZ"];

fn nino_leading_letters(matched: &str) -> (char, char) {
    let mut letters = matched.chars();
    (
        letters.next().unwrap_or('\0'),
        letters.next().unwrap_or('\0'),
    )
}

/// UK NINO (2 letters + 6 digits + 1 suffix letter, no checksum — HMRC
/// structural rule only): rejects a first or second letter from the
/// administratively-unused sets, and a two-letter prefix reserved for
/// temporary/invalid use. The suffix `[A-D]` is regex-owned, so it is never
/// re-checked here (B8a dead-guard lesson).
fn validate_uk_nino(matched: &str) -> bool {
    let (first, second) = nino_leading_letters(matched);
    if NINO_INVALID_FIRST_LETTERS.contains(&first) {
        return false;
    }
    if NINO_INVALID_SECOND_LETTERS.contains(&second) {
        return false;
    }
    let prefix = [first, second].iter().collect::<String>();
    !NINO_INVALID_PREFIXES.contains(&prefix.as_str())
}

const PAN_VALID_ENTITY_CHARS: [char; 10] = ['A', 'B', 'C', 'F', 'G', 'H', 'J', 'L', 'P', 'T'];

/// India PAN (5 letters + 4 digits + 1 letter, no checksum — Income Tax
/// Department structural rule only): valid iff the 4th character (the entity
/// type) is one of the assigned entity codes. The rest of the shape is
/// regex-owned.
fn validate_india_pan(matched: &str) -> bool {
    matched
        .chars()
        .nth(3)
        .is_some_and(|entity_type| PAN_VALID_ENTITY_CHARS.contains(&entity_type))
}

/// Registers every Tier-2 context-gated detector.
pub(super) fn detectors() -> Vec<ContextualDetector> {
    vec![
        ContextualDetector {
            label: "US SSN",
            pattern: Regex::new(r"\b\d{3}[- ]?\d{2}[- ]?\d{4}\b").expect("valid ssn regex"),
            validate: validate_us_ssn,
            context: &["ssn", "social security"],
        },
        ContextualDetector {
            label: "US ITIN",
            pattern: Regex::new(r"\b9\d{2}[- ]?\d{2}[- ]?\d{4}\b").expect("valid itin regex"),
            validate: validate_us_itin,
            context: &["itin", "taxpayer"],
        },
        ContextualDetector {
            label: "UK NINO",
            pattern: Regex::new(r"\b[A-Z]{2} ?\d{2} ?\d{2} ?\d{2} ?[A-D]\b")
                .expect("valid nino regex"),
            validate: validate_uk_nino,
            context: &["nino", "national insurance"],
        },
        ContextualDetector {
            label: "India PAN",
            pattern: Regex::new(r"\b[A-Z]{5}\d{4}[A-Z]\b").expect("valid pan regex"),
            validate: validate_india_pan,
            context: &["pan"],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn validate_us_ssn_accepts_a_structurally_valid_number() {
        assert!(validate_us_ssn("536-90-4399"));
    }

    #[test]
    fn validate_us_ssn_rejects_a_short_digit_run() {
        assert!(!validate_us_ssn("1234"));
    }

    #[test]
    fn validate_us_ssn_rejects_an_all_equal_digit_run() {
        assert!(!validate_us_ssn("111-11-1111"));
    }

    #[test]
    fn validate_us_ssn_rejects_area_000() {
        assert!(!validate_us_ssn("000-12-3456"));
    }

    #[test]
    fn validate_us_ssn_rejects_area_666() {
        assert!(!validate_us_ssn("666-12-3456"));
    }

    #[test]
    fn validate_us_ssn_rejects_area_900_and_above() {
        assert!(!validate_us_ssn("900-12-3456"));
    }

    #[test]
    fn validate_us_ssn_rejects_group_00() {
        assert!(!validate_us_ssn("536-00-4399"));
    }

    #[test]
    fn validate_us_ssn_rejects_serial_0000() {
        assert!(!validate_us_ssn("536-90-0000"));
    }

    #[test]
    fn validate_us_ssn_rejects_the_078051120_placeholder() {
        assert!(!validate_us_ssn("078-05-1120"));
    }

    #[test]
    fn validate_us_ssn_rejects_the_219099999_placeholder() {
        assert!(!validate_us_ssn("219-09-9999"));
    }

    #[test]
    fn validate_us_ssn_rejects_the_123456789_placeholder() {
        assert!(!validate_us_ssn("123-45-6789"));
    }

    #[test]
    fn detectors_registers_all_four_context_gated_detectors_with_their_context_words() {
        let found = detectors();
        assert_eq!(found.len(), 4);
        assert_eq!(found[0].label, "US SSN");
        assert_eq!(found[0].context, &["ssn", "social security"]);
        assert_eq!(found[1].label, "US ITIN");
        assert_eq!(found[1].context, &["itin", "taxpayer"]);
        assert_eq!(found[2].label, "UK NINO");
        assert_eq!(found[2].context, &["nino", "national insurance"]);
        assert_eq!(found[3].label, "India PAN");
        assert_eq!(found[3].context, &["pan"]);
    }

    #[test]
    fn validate_us_itin_accepts_a_group_in_the_70_to_88_range() {
        assert!(validate_us_itin("900-70-1234"));
    }

    /// `900-68-1234` differs from the accepted `900-70-1234` vector only in
    /// the group digits (`68` instead of `70`) — the group-range guard is the
    /// only reason this vector is rejected.
    #[test]
    fn validate_us_itin_rejects_a_group_outside_every_assigned_range() {
        assert!(!validate_us_itin("900-68-1234"));
    }

    #[test]
    fn validate_us_itin_rejects_an_all_equal_digit_run() {
        assert!(!validate_us_itin("999-99-9999"));
    }

    #[test]
    fn validate_uk_nino_accepts_a_structurally_valid_number() {
        assert!(validate_uk_nino("AB123456C"));
    }

    /// `DA123456C` differs from the accepted `AB123456C` vector only in the
    /// first letter (`D` instead of `A`) — the first-letter guard is the only
    /// reason this vector is rejected.
    #[test]
    fn validate_uk_nino_rejects_a_first_letter_from_the_excluded_set() {
        assert!(!validate_uk_nino("DA123456C"));
    }

    /// `AO123456C` keeps an unexcluded first letter and an unexcluded prefix,
    /// differing from the accepted vector only in the second letter (`O`
    /// instead of `B`) — the second-letter guard is the only reason this
    /// vector is rejected.
    #[test]
    fn validate_uk_nino_rejects_a_second_letter_from_the_excluded_set() {
        assert!(!validate_uk_nino("AO123456C"));
    }

    /// `BG123456C` keeps both individual letters outside their respective
    /// excluded sets (`B` is not an excluded first letter, `G` is not an
    /// excluded second letter) — the prefix-exclusion guard is the only
    /// reason this vector is rejected.
    #[test]
    fn validate_uk_nino_rejects_a_reserved_two_letter_prefix() {
        assert!(!validate_uk_nino("BG123456C"));
    }

    #[test]
    fn validate_india_pan_accepts_a_valid_entity_type_character() {
        assert!(validate_india_pan("ABCPD1234E"));
    }

    /// `ABCXD1234E` differs from the accepted `ABCPD1234E` vector only in the
    /// 4th character (`X` instead of `P`) — the entity-type guard is the
    /// only reason this vector is rejected.
    #[test]
    fn validate_india_pan_rejects_an_unassigned_entity_type_character() {
        assert!(!validate_india_pan("ABCXD1234E"));
    }

    #[test]
    fn collect_pii_violations_flags_a_valid_itin_with_nearby_context_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Employee ITIN: 900-70-1234 on file.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert_eq!(out.len(), 1);
        assert!(out[0].1.contains("US ITIN"));
        assert!(!out[0].1.contains("900-70-1234"));
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_valid_itin_with_no_context_word() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Order 900-70-1234 shipped.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }

    #[test]
    fn collect_pii_violations_flags_a_valid_nino_with_nearby_context_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "NINO: AB123456C on file.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert_eq!(out.len(), 1);
        assert!(out[0].1.contains("UK NINO"));
        assert!(!out[0].1.contains("AB123456C"));
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_valid_nino_with_no_context_word() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Ref AB123456C only.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }

    #[test]
    fn collect_pii_violations_flags_a_valid_pan_with_nearby_context_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "PAN ABCPD1234E on file.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert_eq!(out.len(), 1);
        assert!(out[0].1.contains("India PAN"));
        assert!(!out[0].1.contains("ABCPD1234E"));
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_valid_pan_with_no_context_word() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Code ABCPD1234E only.";
        let mut out = Vec::new();

        super::super::collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }
}
