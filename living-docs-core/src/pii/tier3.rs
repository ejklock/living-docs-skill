//! Tier-3 detectors (ADR 0012): the highest-false-positive class, invoked
//! only when the command layer opts in (`--check-tier3`) — never part of the
//! default `collect_pii_violations` scan. Same two-stage regex+validator
//! shape as the Tier-1 detectors (`apac`, `brazil`, `europe`, `financial`).

use super::checksum;
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

/// The regex pins the 5+3-digit shape (with an optional hyphen), so the
/// validator does not re-check length or digit count — a second guard over
/// territory the regex already owns would be dead code (B8a lesson). CEP has
/// no check digit, so the only honest discriminator is rejecting the
/// all-equal placeholder (`00000-000`) that stands in for "no CEP entered"
/// rather than a real postal code.
fn validate_cep(matched: &str) -> bool {
    !checksum::all_same(&checksum::digits(matched))
}

/// The two trailing letters of a UK postcode are its inward code, and
/// official rules never draw those two letters from `C`, `I`, `K`, `M`, `O`,
/// or `V` (they are reserved to avoid confusion with digits or other
/// letters). The regex pins the outward+inward shape but is permissive on
/// letter identity, so this is the one constraint it cannot pin — re-checking
/// it here is not a dead guard (B8a lesson) because the regex genuinely
/// leaves it open. The regex is uppercase-only by design, mirroring the
/// catalog/Presidio `uk_postcode` source, so a lowercase postcode is a known
/// non-match rather than a validator gap.
const UK_FORBIDDEN_INWARD: &[char] = &['C', 'I', 'K', 'M', 'O', 'V'];

fn validate_uk_postcode(matched: &str) -> bool {
    let letters: Vec<char> = matched
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect();
    match letters.as_slice() {
        [.., a, b] => !UK_FORBIDDEN_INWARD.contains(a) && !UK_FORBIDDEN_INWARD.contains(b),
        _ => false,
    }
}

/// Shared by the crypto-address validators below: true iff every character
/// of `body` is identical, or `body` is empty. A single-repeated-character
/// body is how a null/burn/placeholder address is spelled (Bitcoin's
/// all-`1` "eater" address, Ethereum's all-zero null address) — this is the
/// one discriminator available without running the real cryptographic
/// checksum (see `validate_btc_address`/`validate_eth_address` for why the
/// checksum itself is out of scope).
fn all_same_chars(body: &str) -> bool {
    let mut chars = body.chars();
    match chars.next() {
        Some(first) => chars.all(|c| c == first),
        None => true,
    }
}

/// The regex pins both the Bech32 (`bc1...`) and legacy (version byte +
/// Base58 body) shapes and their character sets, so the validator does not
/// re-check length or charset — a second guard over territory the regex
/// already owns would be dead code (B8a lesson). A real Base58Check/Bech32
/// checksum needs SHA-256 (double-SHA256 for Base58Check, BCH for Bech32),
/// a dependency outside this tool's determinism boundary, so the
/// deterministic discriminator here is rejecting a single-repeated-character
/// body — the shape a null/burn/placeholder Bitcoin address takes.
fn validate_btc_address(matched: &str) -> bool {
    let body = matched.strip_prefix("bc1").unwrap_or(&matched[1..]);
    !all_same_chars(body)
}

/// The regex pins the `0x` + 40-hex-digit shape, so the validator does not
/// re-check length or charset (B8a lesson). A real EIP-55 checksum needs
/// Keccak-256, a dependency outside this tool's determinism boundary, so
/// case is deliberately NOT used as a discriminator here — an unchecksummed
/// all-lowercase or all-uppercase address with a varied body is a valid
/// address and must be accepted. The deterministic discriminator is
/// rejecting a single-repeated-character body, the shape Ethereum's null
/// address (`0x000...0`) takes.
fn validate_eth_address(matched: &str) -> bool {
    matched.get(2..).is_some_and(|body| !all_same_chars(body))
}

/// Registers IPv4, IPv6, MAC, CEP, UK postcode, Bitcoin address, and
/// Ethereum address for this slice — Tier-3's per-detector false-positive
/// risk is why each addition here is deliberate rather than batched, unlike
/// the Tier-1 modules that ship a whole region's classes at once.
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
        PiiDetector {
            label: "CEP",
            pattern: Regex::new(r"\b\d{5}-?\d{3}\b").expect("valid cep regex"),
            validate: validate_cep,
        },
        PiiDetector {
            label: "UK postcode",
            pattern: Regex::new(r"\b[A-Z]{1,2}\d[A-Z\d]? ?\d[A-Z]{2}\b")
                .expect("valid uk postcode regex"),
            validate: validate_uk_postcode,
        },
        PiiDetector {
            label: "Bitcoin address",
            pattern: Regex::new(r"\b(bc1[a-z0-9]{25,90}|[13][a-km-zA-HJ-NP-Z1-9]{25,34})\b")
                .expect("valid bitcoin address regex"),
            validate: validate_btc_address,
        },
        PiiDetector {
            label: "Ethereum address",
            pattern: Regex::new(r"\b0x[a-fA-F0-9]{40}\b").expect("valid ethereum address regex"),
            validate: validate_eth_address,
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
    fn validate_cep_accepts_a_hyphenated_cep() {
        assert!(validate_cep("01310-100"));
    }

    #[test]
    fn validate_cep_accepts_an_unhyphenated_cep() {
        assert!(validate_cep("01310100"));
    }

    #[test]
    fn validate_cep_rejects_the_all_equal_placeholder() {
        assert!(!validate_cep("00000-000"));
    }

    #[test]
    fn validate_uk_postcode_accepts_a_two_letter_outward_postcode() {
        assert!(validate_uk_postcode("SW1A 1AA"));
    }

    #[test]
    fn validate_uk_postcode_accepts_another_two_letter_outward_postcode() {
        assert!(validate_uk_postcode("EC1A 1BB"));
    }

    #[test]
    fn validate_uk_postcode_accepts_a_one_letter_outward_postcode() {
        assert!(validate_uk_postcode("M1 1AE"));
    }

    #[test]
    fn validate_uk_postcode_rejects_a_forbidden_inward_letter_pair() {
        assert!(!validate_uk_postcode("SW1A 1CV"));
    }

    #[test]
    fn validate_uk_postcode_rejects_a_forbidden_first_inward_letter() {
        assert!(!validate_uk_postcode("SW1A 1CA"));
    }

    #[test]
    fn validate_uk_postcode_rejects_a_forbidden_second_inward_letter() {
        assert!(!validate_uk_postcode("SW1A 1AV"));
    }

    #[test]
    fn detectors_registers_a_detector_for_each_of_the_seven_tier3_classes() {
        assert_eq!(detectors().len(), 7);
        let labels: Vec<&str> = detectors().iter().map(|d| d.label).collect();
        assert!(labels.contains(&"IPv6 address"));
        assert!(labels.contains(&"MAC address"));
        assert!(labels.contains(&"CEP"));
        assert!(labels.contains(&"UK postcode"));
        assert!(labels.contains(&"Bitcoin address"));
        assert!(labels.contains(&"Ethereum address"));
    }

    #[test]
    fn validate_btc_address_accepts_a_bech32_address() {
        assert!(validate_btc_address(
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
        ));
    }

    #[test]
    fn validate_btc_address_accepts_a_legacy_base58_address() {
        assert!(validate_btc_address("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2"));
    }

    #[test]
    fn validate_btc_address_rejects_a_repeated_character_body() {
        assert!(!validate_btc_address("111111111111111111111111111111"));
    }

    #[test]
    fn validate_eth_address_accepts_a_varied_hex_address() {
        assert!(validate_eth_address(
            "0x52908400098527886E0F7030069857D2E4169EE7"
        ));
    }

    #[test]
    fn validate_eth_address_rejects_the_null_address() {
        assert!(!validate_eth_address(
            "0x0000000000000000000000000000000000000000"
        ));
    }

    #[test]
    fn validate_eth_address_accepts_an_all_lowercase_unchecksummed_address() {
        assert!(validate_eth_address(
            "0x52908400098527886e0f7030069857d2e4169ee7"
        ));
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

    #[test]
    fn collect_tier3_violations_flags_a_cep_and_a_uk_postcode_and_masks_both() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Ship to CEP 01310-100 or the UK branch at SW1A 1AA.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(out
            .iter()
            .any(|(_, message)| message.contains("CEP") && !message.contains("01310-100")));
        assert!(out
            .iter()
            .any(|(_, message)| message.contains("UK postcode") && !message.contains("SW1A 1AA")));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_all_equal_cep_placeholder() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Default CEP on file: 00000-000 (unset).";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(!out.iter().any(|(_, message)| message.contains("CEP")));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_a_forbidden_inward_letter_postcode() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Placeholder address: SW1A 1CV (do not mail).";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(!out
            .iter()
            .any(|(_, message)| message.contains("UK postcode")));
    }

    #[test]
    fn collect_tier3_violations_flags_a_bitcoin_and_an_ethereum_address_and_masks_both() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Send BTC to bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4 or \
                         ETH to 0x52908400098527886E0F7030069857D2E4169EE7.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(out.iter().any(|(_, message)| {
            message.contains("Bitcoin address")
                && !message.contains("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
        }));
        assert!(out.iter().any(|(_, message)| {
            message.contains("Ethereum address")
                && !message.contains("0x52908400098527886E0F7030069857D2E4169EE7")
        }));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_the_bitcoin_and_ethereum_placeholders() {
        let path = Path::new("adr/0001-doc.md");
        let contents = "Eater address: 111111111111111111111111111111. \
                         Null address: 0x0000000000000000000000000000000000000000.";
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        assert!(!out
            .iter()
            .any(|(_, message)| message.contains("Bitcoin address")));
        assert!(!out
            .iter()
            .any(|(_, message)| message.contains("Ethereum address")));
    }
}
