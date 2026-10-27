//! Topic matching for `read --topic`: terms combine with AND, a term matches a
//! word by prefix after accent folding, and matches rank by weighted term
//! frequency. No stemming, no substring hits.

use crate::paths::transliterate;
use crate::record::ExtractedRecord;

const TITLE_WEIGHT: usize = 3;
const DESCRIPTION_WEIGHT: usize = 2;
const BODY_WEIGHT: usize = 1;

/// Splits a topic into accent-folded, lowercased terms.
pub(super) fn terms(topic: &str) -> Vec<String> {
    topic.split_whitespace().map(fold).collect()
}

/// The folded words of a record's title, description and body, each paired
/// with its ranking weight, computed once so matching and ranking share it.
pub(super) struct Folded([(Vec<String>, usize); 3]);

/// Folds a record's searchable fields into lowercase, accent-free words.
pub(super) fn fold_record(record: &ExtractedRecord) -> Folded {
    Folded([
        (words_of(&record.title), TITLE_WEIGHT),
        (words_of(&record.description), DESCRIPTION_WEIGHT),
        (words_of(&record.body), BODY_WEIGHT),
    ])
}

/// Folds `record` only when there are `terms` to match: with none every record
/// matches and scores 0, so the words are never read.
pub(super) fn fold_for(record: &ExtractedRecord, terms: &[String]) -> Folded {
    if terms.is_empty() {
        return Folded(Default::default());
    }
    fold_record(record)
}

/// Whether every term starts some word in the title, description or body.
pub(super) fn matches(folded: &Folded, terms: &[String]) -> bool {
    terms.iter().all(|term| {
        folded
            .0
            .iter()
            .any(|(words, _)| words.iter().any(|w| w.starts_with(term)))
    })
}

/// Weighted count of words that start with a term: title words weigh most.
pub(super) fn score(folded: &Folded, terms: &[String]) -> usize {
    folded
        .0
        .iter()
        .map(|(words, weight)| weight * hits(words, terms))
        .sum()
}

fn hits(words: &[String], terms: &[String]) -> usize {
    words
        .iter()
        .map(|word| {
            terms
                .iter()
                .filter(|t| word.starts_with(t.as_str()))
                .count()
        })
        .sum()
}

fn words_of(text: &str) -> Vec<String> {
    fold(text)
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_string)
        .collect()
}

fn fold(text: &str) -> String {
    transliterate(text).to_lowercase()
}
