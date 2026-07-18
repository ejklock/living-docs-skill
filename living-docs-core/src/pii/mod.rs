//! Two-stage PII detection (ADR 0012, research note 0001): a permissive
//! `\b`-anchored regex finds a syntactic candidate, then a Rust validator
//! applies separator normalization, trivial-input rejection, and the
//! checksum. Only a checksum-valid match is reported — the checksum is the
//! false-positive filter, not the regex (a broken check digit never fires).
//! Tier-2 detectors (`contextual`) extend this to identifiers with no
//! checksum: a nearby context word stands in for the checksum as the
//! false-positive filter (see `contextual` for why that needs a distinct
//! detector type).

mod apac;
mod brazil;
mod checksum;
mod contextual;
mod europe;
mod financial;

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// How many characters of surrounding text a Tier-2 detector's context
/// words are searched in, on each side of the match (symmetric).
const CONTEXT_WINDOW: usize = 40;

/// A two-stage detector: `pattern` finds every syntactic candidate,
/// `validate` decides whether one is checksum-valid PII, and `label` names
/// the document class in a reported violation.
struct PiiDetector {
    label: &'static str,
    pattern: Regex,
    validate: fn(&str) -> bool,
}

fn pii_detectors() -> &'static [PiiDetector] {
    static DETECTORS: OnceLock<Vec<PiiDetector>> = OnceLock::new();
    DETECTORS.get_or_init(|| {
        let mut detectors = brazil::detectors();
        detectors.extend(financial::detectors());
        detectors.extend(europe::detectors());
        detectors.extend(apac::detectors());
        detectors
    })
}

fn contextual_detectors() -> &'static [contextual::ContextualDetector] {
    static DETECTORS: OnceLock<Vec<contextual::ContextualDetector>> = OnceLock::new();
    DETECTORS.get_or_init(contextual::detectors)
}

/// The byte offset, in `contents`, of the earliest character to include in a
/// context-word search window that ends at `match_start` — up to
/// `CONTEXT_WINDOW` characters back, found via `char_indices` so the cut
/// always lands on a character boundary, never inside a multi-byte one.
fn window_start(contents: &str, match_start: usize) -> usize {
    contents[..match_start]
        .char_indices()
        .rev()
        .nth(CONTEXT_WINDOW - 1)
        .map_or(0, |(byte_index, _)| byte_index)
}

/// The byte offset, in `contents`, one past the last character to include in
/// a context-word search window that starts at `match_end` — up to
/// `CONTEXT_WINDOW` characters forward, found via `char_indices` for the
/// same char-boundary-safety reason as `window_start`.
fn window_end(contents: &str, match_end: usize) -> usize {
    contents[match_end..]
        .char_indices()
        .nth(CONTEXT_WINDOW)
        .map_or(contents.len(), |(byte_index, _)| match_end + byte_index)
}

/// True iff any of `words` (ASCII-lowercase) occurs, ASCII-case-insensitively,
/// within `CONTEXT_WINDOW` characters before `match_start` or after
/// `match_end` — the proximity gate that lets a Tier-2 detector fire only
/// near a corroborating label.
fn context_word_near(contents: &str, match_start: usize, match_end: usize, words: &[&str]) -> bool {
    let window = contents[window_start(contents, match_start)..window_end(contents, match_end)]
        .to_ascii_lowercase();
    words.iter().any(|word| window.contains(word))
}

fn push_masked_violation(
    out: &mut Vec<(PathBuf, String)>,
    path: &Path,
    label: &str,
    matched: &str,
) {
    out.push((
        path.to_path_buf(),
        format!("{label} detected (masked: {})", mask_pii(matched)),
    ));
}

/// Scans `contents` with every registered detector and pushes a masked
/// violation for each valid match found at `path`: Tier-1 detectors fire
/// unconditionally on a checksum-valid match; Tier-2 detectors additionally
/// require a context word within `CONTEXT_WINDOW` characters of the match.
pub fn collect_pii_violations(path: &Path, contents: &str, out: &mut Vec<(PathBuf, String)>) {
    collect_tier1_violations(path, contents, out);
    collect_tier2_violations(path, contents, out);
}

/// Tier-1 pass: pushes a masked violation for every checksum-valid match of
/// a registered `PiiDetector`, unconditionally (no context gate).
fn collect_tier1_violations(path: &Path, contents: &str, out: &mut Vec<(PathBuf, String)>) {
    for detector in pii_detectors() {
        for candidate in detector.pattern.find_iter(contents) {
            if !(detector.validate)(candidate.as_str()) {
                continue;
            }
            push_masked_violation(out, path, detector.label, candidate.as_str());
        }
    }
}

/// Tier-2 pass: pushes a masked violation for every checksum-valid match of
/// a registered `ContextualDetector` that also has a context word within
/// `CONTEXT_WINDOW` characters of the match (see `context_word_near`).
fn collect_tier2_violations(path: &Path, contents: &str, out: &mut Vec<(PathBuf, String)>) {
    for detector in contextual_detectors() {
        for candidate in detector.pattern.find_iter(contents) {
            if !(detector.validate)(candidate.as_str()) {
                continue;
            }
            if !context_word_near(
                contents,
                candidate.start(),
                candidate.end(),
                detector.context,
            ) {
                continue;
            }
            push_masked_violation(out, path, detector.label, candidate.as_str());
        }
    }
}

/// Masks a matched PII value so the leak gate never echoes it verbatim (a
/// gate that prints the PII it caught would itself be a leak vector): keeps
/// the last 2 characters, replacing every earlier character with `*`.
fn mask_pii(matched: &str) -> String {
    let len = matched.chars().count();
    let keep_from = len.saturating_sub(2);
    matched
        .chars()
        .enumerate()
        .map(|(i, c)| if i < keep_from { '*' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_pii_keeps_only_the_last_two_characters() {
        let matched = "111.444.777-35";
        let masked = mask_pii(matched);
        let expected_stars = "*".repeat(matched.chars().count() - 2);
        assert_eq!(masked, format!("{expected_stars}35"));
    }

    #[test]
    fn mask_pii_preserves_the_input_length() {
        let matched = "12.ABC.345/01DE-35";
        assert_eq!(mask_pii(matched).chars().count(), matched.chars().count());
    }

    #[test]
    fn collect_pii_violations_flags_a_checksum_valid_cpf_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Contact document: 111.444.777-35 on file.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, path);
        assert!(out[0].1.contains("Brazilian CPF"));
        assert!(!out[0].1.contains("111.444.777-35"));
        assert!(out[0].1.contains("35"));
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_broken_check_digit() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Contact document: 111.444.777-00 on file.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }

    #[test]
    fn collect_pii_violations_labels_a_checksum_valid_iban() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Wire to IBAN GB82WEST12345698765432 today.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out.iter().any(|(_, message)| message.contains("IBAN")));
    }

    #[test]
    fn collect_pii_violations_labels_a_checksum_valid_dutch_bsn() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "BSN op file: 111222333.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out.iter().any(|(_, message)| message.contains("Dutch BSN")));
    }

    #[test]
    fn collect_pii_violations_labels_a_checksum_valid_south_africa_id() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "ID number on file: 8001015009087.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out
            .iter()
            .any(|(_, message)| message.contains("South African ID")));
    }

    #[test]
    fn context_word_near_finds_a_word_adjacent_to_the_match() {
        let contents = "ssn 123456789 filler";
        let match_start = contents.find("123456789").unwrap();
        let match_end = match_start + "123456789".len();

        assert!(context_word_near(
            contents,
            match_start,
            match_end,
            &["ssn"]
        ));
    }

    #[test]
    fn context_word_near_finds_a_word_exactly_at_the_forty_char_boundary_before_the_match() {
        let prefix = format!("z{}", "a".repeat(39));
        assert_eq!(prefix.chars().count(), 40);
        let contents = format!("{prefix}MATCH");
        let match_start = prefix.len();
        let match_end = contents.len();

        assert!(context_word_near(&contents, match_start, match_end, &["z"]));
    }

    #[test]
    fn context_word_near_does_not_find_a_word_one_char_beyond_the_forty_char_boundary_before_the_match(
    ) {
        let prefix = format!("z{}", "a".repeat(40));
        assert_eq!(prefix.chars().count(), 41);
        let contents = format!("{prefix}MATCH");
        let match_start = prefix.len();
        let match_end = contents.len();

        assert!(!context_word_near(
            &contents,
            match_start,
            match_end,
            &["z"]
        ));
    }

    #[test]
    fn context_word_near_finds_a_word_exactly_at_the_forty_char_boundary_after_the_match() {
        let suffix = format!("{}z", "a".repeat(39));
        assert_eq!(suffix.chars().count(), 40);
        let contents = format!("MATCH{suffix}");
        let match_start = 0;
        let match_end = "MATCH".len();

        assert!(context_word_near(&contents, match_start, match_end, &["z"]));
    }

    #[test]
    fn context_word_near_does_not_find_a_word_one_char_beyond_the_forty_char_boundary_after_the_match(
    ) {
        let suffix = format!("{}z", "a".repeat(40));
        assert_eq!(suffix.chars().count(), 41);
        let contents = format!("MATCH{suffix}");
        let match_start = 0;
        let match_end = "MATCH".len();

        assert!(!context_word_near(
            &contents,
            match_start,
            match_end,
            &["z"]
        ));
    }

    #[test]
    fn context_word_near_is_case_insensitive() {
        let contents = "Social Security number 123456789 on file.";
        let match_start = contents.find("123456789").unwrap();
        let match_end = match_start + "123456789".len();

        assert!(context_word_near(
            contents,
            match_start,
            match_end,
            &["social security"]
        ));
    }

    #[test]
    fn context_word_near_does_not_panic_when_the_window_boundary_falls_inside_multi_byte_chars() {
        let contents = format!("{}123456789", "🎉".repeat(100));
        let match_start = contents.find("123456789").unwrap();
        let match_end = contents.len();

        assert!(!context_word_near(
            &contents,
            match_start,
            match_end,
            &["ssn"]
        ));
    }

    #[test]
    fn context_word_near_finds_a_word_across_non_ascii_surrounding_text() {
        let contents = format!("{} ssn 123456789", "🎉".repeat(45));
        let match_start = contents.find("123456789").unwrap();
        let match_end = contents.len();

        assert!(context_word_near(
            &contents,
            match_start,
            match_end,
            &["ssn"]
        ));
    }

    #[test]
    fn collect_pii_violations_flags_a_valid_ssn_with_nearby_context_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Employee SSN: 536-90-4399 on file.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert_eq!(out.len(), 1);
        assert!(out[0].1.contains("US SSN"));
        assert!(!out[0].1.contains("536-90-4399"));
        assert!(out[0].1.contains("99"));
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_valid_ssn_with_no_context_word() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Order 536-90-4399 shipped.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }

    #[test]
    fn collect_pii_violations_stays_quiet_on_a_structurally_invalid_ssn_even_with_context() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "SSN 000-12-3456 on file.";
        let mut out = Vec::new();

        collect_pii_violations(path, contents, &mut out);

        assert!(out.is_empty());
    }
}
