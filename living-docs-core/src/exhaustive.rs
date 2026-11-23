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

/// Calls `visit` on every ordering of `items`, each item used once, and
/// returns how many orderings it visited: the factorial of the item count.
pub fn for_each_permutation<T: Clone>(items: &[T], visit: &mut impl FnMut(&[T])) -> usize {
    let mut order = items.to_vec();
    permute(&mut order, 0, visit)
}

fn permute<T: Clone>(order: &mut Vec<T>, fixed: usize, visit: &mut impl FnMut(&[T])) -> usize {
    if fixed + 1 >= order.len() {
        visit(order);
        return 1;
    }
    let mut visited = 0;
    for pick in fixed..order.len() {
        order.swap(fixed, pick);
        visited += permute(order, fixed + 1, visit);
        order.swap(fixed, pick);
    }
    visited
}

#[test]
fn every_ordering_of_the_items_is_visited_once() {
    let mut seen = Vec::new();
    let visited = for_each_permutation(&[1, 2, 3], &mut |order| seen.push(order.to_vec()));
    assert_eq!(visited, 6);
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 6);
    assert_eq!(for_each_permutation::<u8>(&[], &mut |_| {}), 1);
}
