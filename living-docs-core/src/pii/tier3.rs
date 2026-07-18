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

/// Shared by CEP, India Voter ID, and Nigeria NIN: each regex pins its own
/// shape and character set, so this validator does not re-check length,
/// digit count, or letter placement — a second guard over territory the
/// regex already owns would be dead code (B8a lesson). None of these three
/// national IDs carries a check digit, so the only honest discriminator
/// available is rejecting an all-identical-digit body (`00000-000`,
/// `ABC0000000`, `00000000000`) — the placeholder shape a stub or "no ID
/// entered" value takes, rather than a real identifier.
fn digits_not_all_same(matched: &str) -> bool {
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

/// São Paulo's RG carries the only nationwide-documented RG check digit
/// (catalog docs/research/0001 line 247); the regex owns the 8-base-digit
/// 2-3-3 shape plus the verifier character, so this validator does exactly
/// two things the regex cannot: reject an all-identical base and verify the
/// mod-11 check digit. The all-identical-base guard is NOT dead code even
/// though the checksum runs afterward — an all-zero base (`000000000`)
/// computes a checksum-valid DV of `0` and would otherwise pass, so the
/// guard catches the one placeholder shape the math alone cannot reject.
/// DV convention mirrors the catalog source: `10` maps to `X`
/// (case-insensitive), `11` maps to `0`, anything else is the digit itself.
fn validate_rg_sp(matched: &str) -> bool {
    let stripped: Vec<char> = matched
        .chars()
        .filter(|c| c.is_ascii_digit() || c.eq_ignore_ascii_case(&'x'))
        .collect();
    if stripped.len() != 9 {
        return false;
    }
    let base_str: String = stripped[..8].iter().collect();
    let base = checksum::digits(&base_str);
    if base.len() != 8 || checksum::all_same(&base) {
        return false;
    }
    rg_sp_verifier_matches(rg_sp_check_digit(&base), stripped[8])
}

/// Weights `2..9` ascending over the 8 base digits (leftmost x2 .. eighth
/// x9), per the catalog oracle — extracted so `validate_rg_sp` stays within
/// the complexity budget.
fn rg_sp_check_digit(base: &[u32]) -> u32 {
    const WEIGHTS: [u32; 8] = [2, 3, 4, 5, 6, 7, 8, 9];
    let sum: u32 = base
        .iter()
        .zip(WEIGHTS)
        .map(|(value, weight)| value * weight)
        .sum();
    11 - (sum % 11)
}

fn rg_sp_verifier_matches(expected_dv: u32, verifier: char) -> bool {
    match expected_dv {
        10 => verifier.eq_ignore_ascii_case(&'X'),
        11 => verifier == '0',
        digit => verifier.to_digit(10) == Some(digit),
    }
}

/// Registers IPv4, IPv6, MAC, CEP, UK postcode, Bitcoin address, Ethereum
/// address, Brazil RG (SP), India Voter ID, and Nigeria NIN for this slice —
/// Tier-3's per-detector false-positive risk is why each addition here is
/// deliberate rather than batched, unlike the Tier-1 modules that ship a
/// whole region's classes at once.
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
            validate: digits_not_all_same,
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
        PiiDetector {
            label: "Brazil RG (SP)",
            pattern: Regex::new(r"\b\d{2}\.?\d{3}\.?\d{3}-?[\dXx]\b").expect("valid rg sp regex"),
            validate: validate_rg_sp,
        },
        PiiDetector {
            label: "India Voter ID",
            pattern: Regex::new(r"\b[A-Z]{3}\d{7}\b").expect("valid india voter id regex"),
            validate: digits_not_all_same,
        },
        PiiDetector {
            label: "Nigeria NIN",
            pattern: Regex::new(r"\b\d{11}\b").expect("valid nigeria nin regex"),
            validate: digits_not_all_same,
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
        assert!(digits_not_all_same("01310-100"));
    }

    #[test]
    fn validate_cep_accepts_an_unhyphenated_cep() {
        assert!(digits_not_all_same("01310100"));
    }

    #[test]
    fn validate_cep_rejects_the_all_equal_placeholder() {
        assert!(!digits_not_all_same("00000-000"));
    }

    #[test]
    fn digits_not_all_same_accepts_a_well_formed_india_voter_id() {
        assert!(digits_not_all_same("ABC1234567"));
    }

    #[test]
    fn digits_not_all_same_rejects_an_india_voter_id_with_an_all_equal_digit_body() {
        assert!(!digits_not_all_same("ABC0000000"));
    }

    #[test]
    fn digits_not_all_same_accepts_a_well_formed_nigeria_nin() {
        assert!(digits_not_all_same("12345678901"));
    }

    #[test]
    fn digits_not_all_same_rejects_an_all_equal_nigeria_nin_placeholder() {
        assert!(!digits_not_all_same("00000000000"));
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
    fn detectors_registers_a_detector_for_each_of_the_ten_tier3_classes() {
        assert_eq!(detectors().len(), 10);
        let labels: Vec<&str> = detectors().iter().map(|d| d.label).collect();
        assert!(labels.contains(&"IPv6 address"));
        assert!(labels.contains(&"MAC address"));
        assert!(labels.contains(&"CEP"));
        assert!(labels.contains(&"UK postcode"));
        assert!(labels.contains(&"Bitcoin address"));
        assert!(labels.contains(&"Ethereum address"));
        assert!(labels.contains(&"Brazil RG (SP)"));
        assert!(labels.contains(&"India Voter ID"));
        assert!(labels.contains(&"Nigeria NIN"));
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

    fn tier3_messages(contents: &str) -> Vec<String> {
        let path = Path::new("adr/0001-doc.md");
        let mut out = Vec::new();

        super::super::collect_tier3_violations(path, contents, &mut out);

        out.into_iter().map(|(_, message)| message).collect()
    }

    fn flags_masked(contents: &str, label: &str, raw: &str) -> bool {
        tier3_messages(contents)
            .iter()
            .any(|message| message.contains(label) && !message.contains(raw))
    }

    fn stays_quiet(contents: &str, label: &str) -> bool {
        !tier3_messages(contents)
            .iter()
            .any(|message| message.contains(label))
    }

    #[test]
    fn collect_tier3_violations_flags_a_valid_ipv6_and_masks_it() {
        let contents = "Host reachable at 2001:db8::1 on the lab network.";

        assert!(flags_masked(contents, "IPv6 address", "2001:db8::1"));
    }

    #[test]
    fn collect_tier3_violations_flags_a_valid_mac_and_masks_it() {
        let contents = "NIC address is 01:23:45:67:89:AB per the asset log.";

        assert!(flags_masked(contents, "MAC address", "01:23:45:67:89:AB"));
    }

    #[test]
    fn collect_tier3_violations_flags_both_an_ipv6_and_a_mac_in_the_same_document() {
        let contents = "IPv6: 2001:db8::1, MAC: 01:23:45:67:89:AB";
        let messages = tier3_messages(contents);

        assert!(messages
            .iter()
            .any(|message| message.contains("IPv6 address")));
        assert!(messages
            .iter()
            .any(|message| message.contains("MAC address")));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_incomplete_ipv6_shape() {
        let contents = "Route: 2001:db8:1:1:1:1:1 unreachable.";

        assert!(stays_quiet(contents, "IPv6 address"));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_all_equal_mac_placeholder() {
        let contents = "Default NIC: 00:00:00:00:00:00 (unset).";

        assert!(stays_quiet(contents, "MAC address"));
    }

    #[test]
    fn collect_tier3_violations_flags_a_cep_and_a_uk_postcode_and_masks_both() {
        let contents = "Ship to CEP 01310-100 or the UK branch at SW1A 1AA.";

        assert!(flags_masked(contents, "CEP", "01310-100"));
        assert!(flags_masked(contents, "UK postcode", "SW1A 1AA"));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_an_all_equal_cep_placeholder() {
        let contents = "Default CEP on file: 00000-000 (unset).";

        assert!(stays_quiet(contents, "CEP"));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_a_forbidden_inward_letter_postcode() {
        let contents = "Placeholder address: SW1A 1CV (do not mail).";

        assert!(stays_quiet(contents, "UK postcode"));
    }

    #[test]
    fn collect_tier3_violations_flags_a_bitcoin_and_an_ethereum_address_and_masks_both() {
        let contents = "Send BTC to bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4 or \
                         ETH to 0x52908400098527886E0F7030069857D2E4169EE7.";

        assert!(flags_masked(
            contents,
            "Bitcoin address",
            "bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4"
        ));
        assert!(flags_masked(
            contents,
            "Ethereum address",
            "0x52908400098527886E0F7030069857D2E4169EE7"
        ));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_the_bitcoin_and_ethereum_placeholders() {
        let contents = "Eater address: 111111111111111111111111111111. \
                         Null address: 0x0000000000000000000000000000000000000000.";

        assert!(stays_quiet(contents, "Bitcoin address"));
        assert!(stays_quiet(contents, "Ethereum address"));
    }

    #[test]
    fn validate_rg_sp_accepts_a_dotted_valid_rg() {
        assert!(validate_rg_sp("12.345.678-2"));
    }

    #[test]
    fn validate_rg_sp_accepts_an_undotted_valid_rg() {
        assert!(validate_rg_sp("123456782"));
    }

    #[test]
    fn validate_rg_sp_accepts_the_x_verifier_form() {
        assert!(validate_rg_sp("82.345.678-X"));
    }

    #[test]
    fn validate_rg_sp_accepts_an_undotted_lowercase_x_verifier() {
        assert!(validate_rg_sp("82345678x"));
    }

    #[test]
    fn validate_rg_sp_rejects_a_wrong_check_digit() {
        assert!(!validate_rg_sp("123456783"));
    }

    #[test]
    fn validate_rg_sp_rejects_the_all_identical_base_placeholder() {
        assert!(!validate_rg_sp("000000000"));
    }

    #[test]
    fn validate_rg_sp_rejects_a_digit_count_mismatch() {
        assert!(!validate_rg_sp("1234567"));
        assert!(!validate_rg_sp("1234567890"));
    }

    #[test]
    fn collect_tier3_violations_flags_a_valid_rg_sp_and_masks_it_while_staying_quiet_on_bad_shapes()
    {
        let contents = "Valid RG on file: 12.345.678-2. Wrong checksum shape: 123456783. \
                         Placeholder shape: 000000000.";

        let rg_matches: Vec<String> = tier3_messages(contents)
            .into_iter()
            .filter(|message| message.contains("Brazil RG (SP)"))
            .collect();

        assert_eq!(rg_matches.len(), 1);
        assert!(!rg_matches[0].contains("12.345.678-2"));
    }

    #[test]
    fn collect_tier3_violations_flags_an_india_voter_id_and_a_nigeria_nin_and_masks_both() {
        let contents = "Voter ID on file: ABC1234567. NIN on file: 12345678901.";

        assert!(flags_masked(contents, "India Voter ID", "ABC1234567"));
        assert!(flags_masked(contents, "Nigeria NIN", "12345678901"));
    }

    #[test]
    fn collect_tier3_violations_stays_quiet_on_the_india_voter_id_and_nigeria_nin_placeholders() {
        let contents = "Placeholder voter ID: ABC0000000. Placeholder NIN: 00000000000.";

        assert!(stays_quiet(contents, "India Voter ID"));
        assert!(stays_quiet(contents, "Nigeria NIN"));
    }
}
