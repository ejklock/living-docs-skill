//! Two-stage PII detection (ADR 0012, research note 0001): a permissive
//! `\b`-anchored regex finds a syntactic candidate, then a Rust validator
//! applies separator normalization, trivial-input rejection, and the
//! checksum. Only a checksum-valid match is reported — the checksum is the
//! false-positive filter, not the regex (a broken check digit never fires).

mod brazil;
mod checksum;

use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The kind of document a `PiiDetector` matches, used to label a reported
/// violation without re-deriving it from the matched text.
#[derive(Clone, Copy)]
pub enum PiiClass {
    Cpf,
    Cnpj,
    Pis,
}

impl PiiClass {
    pub fn label(&self) -> &'static str {
        match self {
            PiiClass::Cpf => "Brazilian CPF",
            PiiClass::Cnpj => "Brazilian CNPJ",
            PiiClass::Pis => "Brazilian PIS/PASEP",
        }
    }
}

/// A two-stage detector: `pattern` finds every syntactic candidate,
/// `validate` decides whether one is checksum-valid PII.
struct PiiDetector {
    class: PiiClass,
    pattern: Regex,
    validate: fn(&str) -> bool,
}

fn pii_detectors() -> &'static [PiiDetector] {
    static DETECTORS: OnceLock<Vec<PiiDetector>> = OnceLock::new();
    DETECTORS.get_or_init(brazil::detectors)
}

/// Scans `contents` with every registered detector and pushes a masked
/// violation for each checksum-valid match found at `path`.
pub fn collect_pii_violations(path: &Path, contents: &str, out: &mut Vec<(PathBuf, String)>) {
    for detector in pii_detectors() {
        for candidate in detector.pattern.find_iter(contents) {
            if !(detector.validate)(candidate.as_str()) {
                continue;
            }
            out.push((
                path.to_path_buf(),
                format!(
                    "{} detected (masked: {})",
                    detector.class.label(),
                    mask_pii(candidate.as_str())
                ),
            ));
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
}
