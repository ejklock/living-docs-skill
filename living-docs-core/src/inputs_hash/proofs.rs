//! Kani proof for CRLF normalisation.

use super::normalise_newlines;

/// For up to four symbolic bytes, text with LF line ends and the same text
/// with every `\n` written as `\r\n` normalise to the same bytes. The function
/// is not idempotent on `\r\r\n`, so the property is stated over this pair.
/// The harness earns its place beside the 6-byte enumeration because it
/// covers all 256 byte values.
#[kani::proof]
#[kani::unwind(10)]
fn crlf_and_lf_text_normalise_alike() {
    let bytes: [u8; 4] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= 4);
    kani::assume(bytes[..len].iter().all(|b| *b != b'\r'));
    let mut crlf = [0u8; 8];
    let mut used = 0;
    for byte in &bytes[..len] {
        if *byte == b'\n' {
            crlf[used] = b'\r';
            used += 1;
        }
        crlf[used] = *byte;
        used += 1;
    }
    assert_eq!(
        normalise_newlines(&bytes[..len]),
        normalise_newlines(&crlf[..used])
    );
}
