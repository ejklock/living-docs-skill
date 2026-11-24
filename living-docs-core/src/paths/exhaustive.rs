//! Every title up to four characters over a mixed alphabet — ASCII letters,
//! digit, separators and non-ASCII letters that transliterate — slugifies into
//! the slug grammar, and slugifying again changes nothing.

use super::slugify;
use crate::exhaustive::for_each_sequence;

const ALPHABET: [char; 10] = ['a', 'Z', '0', '-', ' ', '.', 'é', 'ß', 'İ', '_'];

fn in_slug_grammar(text: &str) -> bool {
    text.bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !text.starts_with('-')
        && !text.ends_with('-')
        && !text.contains("--")
}

#[test]
fn slugify_stays_in_the_slug_grammar_and_is_idempotent_for_every_short_title() {
    let cases = for_each_sequence(&ALPHABET, 4, &mut |chars| {
        let title: String = chars.iter().collect();
        let slug = slugify(&title);
        assert!(in_slug_grammar(&slug), "{title:?} gave {slug:?}");
        assert_eq!(slugify(&slug), slug, "{title:?} is not idempotent");
    });
    assert_eq!(cases, 1 + 10 + 100 + 1000 + 10_000);
}
