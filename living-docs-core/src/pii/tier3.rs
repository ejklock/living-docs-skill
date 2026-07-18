//! Tier-3 detectors (ADR 0012): the highest-false-positive class, invoked
//! only when the command layer opts in (`--check-tier3`) — never part of the
//! default `collect_pii_violations` scan. Same two-stage regex+validator
//! shape as the Tier-1 detectors (`apac`, `brazil`, `europe`, `financial`).

use super::PiiDetector;
use regex::Regex;

/// The regex already pins the dotted-quad shape (four `\d{1,3}` groups
/// separated by `.`), so the validator does not re-check the group count — a
/// second guard over territory the regex already owns would be dead code
/// (B8a South Africa citizenship-flag lesson). Valid iff every octet parses
/// as an integer no greater than 255.
fn validate_ipv4(matched: &str) -> bool {
    matched
        .split('.')
        .all(|octet| octet.parse::<u32>().is_ok_and(|value| value <= 255))
}

/// Registers only IPv4 for this slice — Tier-3's per-detector false-positive
/// risk is why each addition here is deliberate rather than batched, unlike
/// the Tier-1 modules that ship a whole region's classes at once.
pub(super) fn detectors() -> Vec<PiiDetector> {
    vec![PiiDetector {
        label: "IPv4 address",
        pattern: Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("valid ipv4 regex"),
        validate: validate_ipv4,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_ipv4_accepts_a_dotted_quad_with_every_octet_in_range() {
        assert!(validate_ipv4("192.168.1.1"));
    }

    #[test]
    fn validate_ipv4_rejects_an_octet_above_two_hundred_fifty_five() {
        assert!(!validate_ipv4("999.1.1.1"));
    }

    #[test]
    fn detectors_registers_one_detector_for_ipv4() {
        assert_eq!(detectors().len(), 1);
    }
}
