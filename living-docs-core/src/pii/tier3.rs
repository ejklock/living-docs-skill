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

/// The regex pins the colon-group shape; `std::net::Ipv6Addr`'s parser is the
/// discriminator for whether that shape is a real address (right group
/// count, at most one `::`, valid hex per group) — hand-rolling that check
/// here would duplicate a guard the regex+parser already own (B8a lesson).
/// Known gap by design: a leading `::` form (e.g. `::1`) is never matched,
/// because `\b` cannot anchor immediately before a leading colon — this
/// under-detects loopback-style addresses only, acceptable for Tier-3.
fn validate_ipv6(matched: &str) -> bool {
    matched.parse::<std::net::Ipv6Addr>().is_ok()
}

/// The regex pins the six-pair colon/dash shape, so the validator re-checks
/// only what the regex cannot: whether every octet is the same value, the
/// all-equal placeholder (`00:00:00:00:00:00`, `FF-FF-FF-FF-FF-FF`) vendors
/// use to redact or stub a MAC rather than emit a real one.
fn validate_mac(matched: &str) -> bool {
    let mut hex = matched.chars().filter(|c| c.is_ascii_hexdigit());
    let Some(first) = hex.next() else {
        return false;
    };
    hex.any(|c| c != first)
}

/// Registers IPv4, IPv6, and MAC for this slice — Tier-3's per-detector
/// false-positive risk is why each addition here is deliberate rather than
/// batched, unlike the Tier-1 modules that ship a whole region's classes at
/// once.
pub(super) fn detectors() -> Vec<PiiDetector> {
    vec![
        PiiDetector {
            label: "IPv4 address",
            pattern: Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").expect("valid ipv4 regex"),
            validate: validate_ipv4,
        },
        PiiDetector {
            label: "IPv6 address",
            pattern: Regex::new(r"\b[0-9A-Fa-f]{1,4}(?::[0-9A-Fa-f]{0,4}){2,7}\b")
                .expect("valid ipv6 regex"),
            validate: validate_ipv6,
        },
        PiiDetector {
            label: "MAC address",
            pattern: Regex::new(r"\b(?:[0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}\b")
                .expect("valid mac regex"),
            validate: validate_mac,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn validate_ipv4_accepts_a_dotted_quad_with_every_octet_in_range() {
        assert!(validate_ipv4("192.168.1.1"));
    }

    #[test]
    fn validate_ipv4_rejects_an_octet_above_two_hundred_fifty_five() {
        assert!(!validate_ipv4("999.1.1.1"));
    }

    #[test]
    fn validate_ipv6_accepts_a_compressed_address() {
        assert!(validate_ipv6("2001:db8::1"));
    }

    #[test]
    fn validate_ipv6_accepts_a_full_eight_group_address() {
        assert!(validate_ipv6("2001:0db8:85a3:0000:0000:8a2e:0370:7334"));
    }

    #[test]
    fn validate_ipv6_rejects_seven_groups_with_no_double_colon() {
        assert!(!validate_ipv6("2001:db8:1:1:1:1:1"));
    }

    #[test]
    fn validate_mac_accepts_a_colon_separated_address() {
        assert!(validate_mac("01:23:45:67:89:AB"));
    }

    #[test]
    fn validate_mac_accepts_a_dash_separated_address() {
        assert!(validate_mac("01-23-45-67-89-AB"));
    }

    #[test]
    fn validate_mac_rejects_the_all_equal_placeholder() {
        assert!(!validate_mac("00:00:00:00:00:00"));
    }

    #[test]
    fn detectors_registers_a_detector_for_ipv4_ipv6_and_mac() {
        assert_eq!(detectors().len(), 3);
        let labels: Vec<&str> = detectors().iter().map(|d| d.label).collect();
        assert!(labels.contains(&"IPv6 address"));
        assert!(labels.contains(&"MAC address"));
    }

    #[test]
    fn collect_tier3_violations_flags_a_valid_ipv6_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Host reachable at 2001:db8::1 on the lab network.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(out.iter().any(
            |(_, message)| message.contains("IPv6 address") && !message.contains("2001:db8::1")
        ));
    }

    #[test]
    fn collect_tier3_violations_flags_a_valid_mac_and_masks_it() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "NIC address is 01:23:45:67:89:AB per the asset log.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(out
            .iter()
            .any(|(_, message)| message.contains("MAC address")
                && !message.contains("01:23:45:67:89:AB")));
    }

    #[test]
    fn collect_tier3_violations_flags_both_an_ipv6_and_a_mac_in_the_same_document() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "IPv6: 2001:db8::1, MAC: 01:23:45:67:89:AB";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(out
            .iter()
            .any(|(_, message)| message.contains("IPv6 address")));
        assert!(out
            .iter()
            .any(|(_, message)| message.contains("MAC address")));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_incomplete_ipv6_shape() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Route: 2001:db8:1:1:1:1:1 unreachable.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(!out
            .iter()
            .any(|(_, message)| message.contains("IPv6 address")));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_all_equal_mac_placeholder() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Default NIC: 00:00:00:00:00:00 (unset).";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(!out
            .iter()
            .any(|(_, message)| message.contains("MAC address")));
    }
}
