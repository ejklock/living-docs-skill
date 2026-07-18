//! Reusable, exhaustively-tested checksum primitives shared by every
//! country-specific PII validator (ADR 0012). Kept country-agnostic so a
//! new detector never re-derives digit extraction or the mod-11 check-digit
//! rule.

/// Extracts the ASCII digits from `s` as their numeric value (`0..=9`),
/// discarding every non-digit character (separators, letters, whitespace).
pub fn digits(s: &str) -> Vec<u32> {
    s.chars().filter_map(|c| c.to_digit(10)).collect()
}

/// True when every value in `values` is identical — e.g. the ten forbidden
/// all-repeated-digit CPFs, which pass the mod-11 math but are Receita
/// Federal placeholders rather than real documents. An empty slice is never
/// "all same" (there is nothing to reject).
pub fn all_same(values: &[u32]) -> bool {
    match values.first() {
        Some(first) => values.iter().all(|value| value == first),
        None => false,
    }
}

/// The weighted mod-11 check digit shared by CPF, CNPJ, and PIS/PASEP/NIT
/// (research note 0001 §3): `resto = (Σ values[i] * weights[i]) % 11`, then
/// the check digit is `0` when `resto < 2`, else `11 - resto`.
pub fn weighted_mod11_dv(values: &[u32], weights: &[u32]) -> u32 {
    let sum: u32 = values
        .iter()
        .zip(weights)
        .map(|(value, weight)| value * weight)
        .sum();
    let resto = sum % 11;
    if resto < 2 {
        0
    } else {
        11 - resto
    }
}

/// The Luhn (mod-10) check shared by payment cards and the US NPI (research
/// note 0001 §4): doubling every second digit from the rightmost, subtracting
/// 9 from any doubled value over 9, then requiring the total to be a
/// multiple of 10.
pub fn luhn_valid(values: &[u32]) -> bool {
    let sum: u32 = values
        .iter()
        .rev()
        .enumerate()
        .map(|(i, value)| {
            if i % 2 == 1 {
                let doubled = value * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                *value
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_keeps_only_ascii_digits_as_their_numeric_value() {
        assert_eq!(
            digits("111.444.777-35"),
            vec![1, 1, 1, 4, 4, 4, 7, 7, 7, 3, 5]
        );
    }

    #[test]
    fn digits_returns_empty_for_a_string_with_no_digits() {
        assert!(digits("no digits here").is_empty());
    }

    #[test]
    fn all_same_is_true_for_identical_values() {
        assert!(all_same(&[1, 1, 1, 1]));
    }

    #[test]
    fn all_same_is_false_for_differing_values() {
        assert!(!all_same(&[1, 1, 2, 1]));
    }

    #[test]
    fn all_same_is_false_for_an_empty_slice() {
        assert!(!all_same(&[]));
    }

    #[test]
    fn weighted_mod11_dv_matches_the_cpf_worked_example_dv1() {
        let base = digits("111444777");
        let weights = [10, 9, 8, 7, 6, 5, 4, 3, 2];
        assert_eq!(weighted_mod11_dv(&base, &weights), 3);
    }

    #[test]
    fn weighted_mod11_dv_matches_the_cpf_worked_example_dv2() {
        let base = digits("1114447773");
        let weights = [11, 10, 9, 8, 7, 6, 5, 4, 3, 2];
        assert_eq!(weighted_mod11_dv(&base, &weights), 5);
    }

    #[test]
    fn weighted_mod11_dv_matches_the_cnpj_worked_example_dv1() {
        let base = digits("595412640001");
        let weights = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
        assert_eq!(weighted_mod11_dv(&base, &weights), 0);
    }

    #[test]
    fn weighted_mod11_dv_matches_the_pis_worked_example_dv() {
        let base = digits("1200630723");
        let weights = [3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
        assert_eq!(weighted_mod11_dv(&base, &weights), 3);
    }

    #[test]
    fn weighted_mod11_dv_returns_eleven_minus_resto_at_the_resto_equals_two_boundary() {
        assert_eq!(weighted_mod11_dv(&[2], &[1]), 9);
    }

    #[test]
    fn luhn_valid_accepts_a_known_valid_vector() {
        assert!(luhn_valid(&digits("4111111111111111")));
    }

    #[test]
    fn luhn_valid_rejects_a_broken_vector() {
        assert!(!luhn_valid(&digits("4111111111111112")));
    }
}
