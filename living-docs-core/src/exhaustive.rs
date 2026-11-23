//! The enumerator shared by the bounded exhaustive tests: every sequence of
//! up to a maximum length over an alphabet, including the empty one.

/// Calls `visit` on every sequence of at most `max_len` symbols drawn from
/// `alphabet`, shorter sequences first within each prefix, and returns how
/// many sequences it visited.
pub fn for_each_sequence<T: Copy>(
    alphabet: &[T],
    max_len: usize,
    visit: &mut impl FnMut(&[T]),
) -> usize {
    let mut prefix = Vec::with_capacity(max_len);
    extend(alphabet, max_len, &mut prefix, visit)
}

fn extend<T: Copy>(
    alphabet: &[T],
    max_len: usize,
    prefix: &mut Vec<T>,
    visit: &mut impl FnMut(&[T]),
) -> usize {
    visit(prefix);
    let mut visited = 1;
    if prefix.len() < max_len {
        for symbol in alphabet {
            prefix.push(*symbol);
            visited += extend(alphabet, max_len, prefix, visit);
            prefix.pop();
        }
    }
    visited
}

#[test]
fn every_sequence_up_to_the_maximum_length_is_visited_once() {
    let mut seen = Vec::new();
    let visited = for_each_sequence(&['a', 'b'], 2, &mut |seq| seen.push(seq.to_vec()));
    assert_eq!(visited, 7);
    assert_eq!(seen.len(), 7);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 7);
}
