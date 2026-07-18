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

/// Registers every Tier-2 context-gated detector; today, only the US SSN.
pub(super) fn detectors() -> Vec<ContextualDetector> {
    vec![ContextualDetector {
        label: "US SSN",
        pattern: Regex::new(r"\b\d{3}[- ]?\d{2}[- ]?\d{4}\b").expect("valid ssn regex"),
        validate: validate_us_ssn,
        context: &["ssn", "social security"],
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn detectors_registers_the_us_ssn_detector_with_ssn_context_words() {
        let found = detectors();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].label, "US SSN");
        assert_eq!(found[0].context, &["ssn", "social security"]);
    }
}
