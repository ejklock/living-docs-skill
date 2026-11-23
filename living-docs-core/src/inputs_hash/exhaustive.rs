//! Every byte string up to six bytes over `a`, `\r` and `\n` is rewritten
//! exactly as an independent oracle says: each `\r\n` becomes `\n` and every
//! other byte is kept. Text with no `\r` is left unchanged and normalises to
//! the same bytes as the same text with each `\n` written as `\r\n`.

use super::normalise_newlines;
use crate::exhaustive::for_each_sequence;

const ALPHABET: [u8; 3] = [b'a', b'\r', b'\n'];
const MAX_LEN: usize = 6;

/// Replaces each `\r\n` with `\n`, scanning left to right.
fn crlf_to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while let Some((first, tail)) = rest.split_first() {
        if rest.starts_with(b"\r\n") {
            out.push(b'\n');
            rest = &rest[2..];
        } else {
            out.push(*first);
            rest = tail;
        }
    }
    out
}

#[test]
fn normalisation_rewrites_exactly_each_crlf_for_every_short_text() {
    let cases = for_each_sequence(&ALPHABET, MAX_LEN, &mut |bytes| {
        assert_eq!(normalise_newlines(bytes), crlf_to_lf(bytes), "{bytes:?}");
    });
    assert_eq!(cases, 1093);
}

#[test]
fn lf_text_and_its_crlf_form_normalise_alike_for_every_short_text() {
    let mut lf_texts = 0;
    for_each_sequence(&ALPHABET, MAX_LEN, &mut |lf| {
        if lf.contains(&b'\r') {
            return;
        }
        lf_texts += 1;
        let crlf: Vec<u8> = lf
            .iter()
            .flat_map(|b| {
                if *b == b'\n' {
                    vec![b'\r', *b]
                } else {
                    vec![*b]
                }
            })
            .collect();
        assert_eq!(normalise_newlines(lf), lf, "{lf:?} changed");
        assert_eq!(normalise_newlines(lf), normalise_newlines(&crlf), "{lf:?}");
    });
    assert_eq!(lf_texts, 127);
}
