//! In-force ADRs that share rare words with a new decision: plain term
//! overlap over lowercased ASCII letter runs, document frequency decided by
//! integer counts, so the list is byte-stable and never judges a conflict.

use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BTreeSet;

/// The shortest word that counts.
const MIN_WORD_LEN: usize = 4;
/// The fewest rare words two records must share to be listed.
const MIN_SHARED: usize = 2;
/// The most candidates listed.
const MAX_LISTED: usize = 3;

/// Words of four letters or more that carry no topic, never counted.
const STOP_WORDS: &str = "about after again against also always among because been before \
    being between both cannot could does done down during each either else even ever every \
    from further have having here hers herself himself into itself just keep like made make \
    many more most much must never none once only other others otherwise ours over same shall \
    should since some such than that their theirs them then there these they this those though \
    through thus under until upon used uses using very want wants were what when whenever \
    where whether which while whom whose will with within without would your yours";

/// One in-force ADR as the scoring reads it.
pub struct AdrText<'a> {
    pub number: i32,
    pub title: &'a str,
    pub description: &'a str,
    pub body: &'a str,
}

/// An in-force ADR that shares rare words with the query, with those words.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Candidate {
    #[serde(rename = "type")]
    pub doc_type: &'static str,
    pub number: i32,
    pub title: String,
    pub shared: Vec<String>,
}

/// The ASCII letter runs of `text` that are long enough; any other
/// character, a non-ASCII letter included, ends a run.
fn letter_runs(text: &str) -> impl Iterator<Item = &str> {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|run| run.len() >= MIN_WORD_LEN)
}

/// The lowercased letter runs of `text` that are not stop words.
fn words(text: &str) -> BTreeSet<String> {
    letter_runs(text)
        .map(str::to_ascii_lowercase)
        .filter(|word| !STOP_WORDS.split_whitespace().any(|stop| stop == word))
        .collect()
}

/// The most records a word may occur in and still be rare: 3% of `total`
/// rounded down, never fewer than 2.
fn rare_cut(total: usize) -> usize {
    (total * 3 / 100).max(2)
}

/// The query words that occur in `adr`, so only they are kept per record.
fn hits<'q>(query: &'q BTreeSet<String>, adr: &AdrText) -> BTreeSet<&'q str> {
    let mut found = BTreeSet::new();
    let mut lowered = String::new();
    for text in [adr.title, adr.description, adr.body] {
        for run in letter_runs(text) {
            lowered.clear();
            lowered.push_str(run);
            lowered.make_ascii_lowercase();
            found.extend(query.get(lowered.as_str()).map(String::as_str));
        }
    }
    found
}

/// Up to three in-force ADRs sharing at least two rare words with `query`,
/// most shared words first, then by number; each lists its shared words sorted.
pub fn candidates(query: &str, adrs: &[AdrText]) -> Vec<Candidate> {
    let query = words(query);
    let per_adr: Vec<BTreeSet<&str>> = adrs.iter().map(|adr| hits(&query, adr)).collect();
    let cut = rare_cut(adrs.len());
    let rare: BTreeSet<&str> = query
        .iter()
        .map(String::as_str)
        .filter(|word| per_adr.iter().filter(|set| set.contains(word)).count() <= cut)
        .collect();
    let mut found: Vec<Candidate> = adrs
        .iter()
        .zip(&per_adr)
        .filter_map(|(adr, set)| {
            let shared: Vec<String> = set.intersection(&rare).map(|w| w.to_string()).collect();
            (shared.len() >= MIN_SHARED).then(|| Candidate {
                doc_type: "ADR",
                number: adr.number,
                title: adr.title.to_string(),
                shared,
            })
        })
        .collect();
    found.sort_by_key(|c| (Reverse(c.shared.len()), c.number));
    found.truncate(MAX_LISTED);
    found
}

#[cfg(test)]
mod tests;
