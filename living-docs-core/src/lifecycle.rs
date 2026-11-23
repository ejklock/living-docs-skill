//! The record lifecycle as one transition table. `write set status` and
//! `write supersede` both ask it before any file is written, so the two
//! verbs can never disagree about which transitions are legal.
//!
//! The table is pure: it reads the facts of the records a transition
//! touches and answers with the refusal its rule gives, or nothing. Only
//! [`refuse_supersede_at`] reads a store, to gather those facts.

use crate::doc_type::{spec_for_frontmatter, DocTypeSpec};
use crate::record::{extract_record, ExtractedRecord};
use crate::store::DocStore;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::Path;

/// What the table needs to know of one record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Facts {
    pub number: i32,
    pub status: Option<String>,
    /// The raw `superseded_by` value, empty or absent when there is none.
    pub successor: Option<String>,
}

impl Facts {
    fn successor(&self) -> Option<&str> {
        let successor = self.successor.as_deref()?.trim();
        (!successor.is_empty()).then_some(successor)
    }

    fn is_superseded(&self) -> bool {
        let by_status = self
            .status
            .as_deref()
            .is_some_and(|status| status.eq_ignore_ascii_case("superseded"));
        by_status || self.successor().is_some()
    }
}

/// The rule a refused transition broke, with the records involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Refusal {
    /// `set status` on a superseded record.
    StatusOnSuperseded {
        label: String,
        successor: Option<String>,
    },
    /// `supersede` of a record that already has a successor.
    AlreadySuperseded { old: String, by: String },
    /// `supersede` whose successor's chain reaches the record itself.
    Cycle { chain: Vec<String> },
    /// `supersede` by a record that is retired.
    RetiredSuccessor {
        old: String,
        new: String,
        status: String,
    },
}

const REGENERATE: &str = "then run `living-docs manage fmt` and `living-docs manage index`";

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StatusOnSuperseded {
                label,
                successor: Some(by),
            } => write!(
                f,
                "{label} is superseded: superseded_by is {by}, so its status is not changed; to undo the supersede, by hand remove superseded_by from this record and set its status, remove this record's number from supersedes on {by}, {REGENERATE}"
            ),
            Self::StatusOnSuperseded {
                label,
                successor: None,
            } => write!(
                f,
                "{label} has status Superseded, so its status is not changed; to undo the supersede, by hand set its status and remove this record's number from supersedes on its successor, {REGENERATE}"
            ),
            Self::AlreadySuperseded { old, by } => write!(
                f,
                "{old} already superseded by {by}; a record has one successor, so it is not superseded again"
            ),
            Self::Cycle { chain } => write!(
                f,
                "superseding would close a cycle: {}; a supersede chain never returns to a record",
                chain.join(" -> ")
            ),
            Self::RetiredSuccessor { old, new, status } => write!(
                f,
                "{new} cannot supersede {old}: it is retired (status {status}), so the decision would leave the in-force view"
            ),
        }
    }
}

fn label(spec: &DocTypeSpec, number: i32) -> String {
    format!("{} {number:04}", spec.frontmatter)
}

/// Whether `set status` may change the status of `record`: it may not when
/// the record is superseded, by its status or by a named successor.
pub(crate) fn refuse_set_status(spec: &DocTypeSpec, record: &Facts) -> Result<(), Refusal> {
    if !record.is_superseded() {
        return Ok(());
    }
    Err(Refusal::StatusOnSuperseded {
        label: label(spec, record.number),
        successor: record.successor().map(str::to_owned),
    })
}

/// Whether `old` may be superseded by `new`: `old` has no successor yet (or
/// already names `new`, so a repeat changes nothing),
/// following successors from `new` (`successor_of`, by number) never
/// reaches `old`, and `new` is not retired. A `Proposed` successor is legal.
pub(crate) fn refuse_supersede(
    spec: &DocTypeSpec,
    old: &Facts,
    new: &Facts,
    successor_of: &BTreeMap<i32, i32>,
) -> Result<(), Refusal> {
    let old_label = label(spec, old.number);
    if let Some(by) = old.successor() {
        if by.parse() == Ok(new.number) {
            return Ok(());
        }
        let by = by
            .parse()
            .map_or_else(|_| by.to_owned(), |number| label(spec, number));
        return Err(Refusal::AlreadySuperseded { old: old_label, by });
    }
    if let Some(walked) = path_between(new.number, old.number, successor_of) {
        let named = walked.into_iter().map(|number| label(spec, number));
        let chain = std::iter::once(old_label).chain(named).collect();
        return Err(Refusal::Cycle { chain });
    }
    match new.status.as_deref() {
        Some(status) if spec.is_retired(status) => Err(Refusal::RetiredSuccessor {
            old: old_label,
            new: label(spec, new.number),
            status: status.to_owned(),
        }),
        _ => Ok(()),
    }
}

/// The numbers met following successors from `from` up to and including
/// `to`, when the walk reaches `to`. A walk that loops elsewhere ends.
fn path_between(from: i32, to: i32, successor_of: &BTreeMap<i32, i32>) -> Option<Vec<i32>> {
    let mut walked = vec![from];
    while walked.last() != Some(&to) {
        let next = *successor_of.get(walked.last()?)?;
        if walked.contains(&next) {
            return None;
        }
        walked.push(next);
    }
    Some(walked)
}

/// The facts of the record at `path`, read from `store`, with its type.
fn facts_at(store: &dyn DocStore, path: &Path) -> Result<(Facts, String), String> {
    let text = store.read(path).map_err(|e| e.to_string())?;
    let record = extract_record(path, &text);
    let number = record
        .number
        .ok_or_else(|| format!("{}: has no record number", path.display()))?;
    let facts = Facts {
        number,
        status: record.status,
        successor: record.superseded_by,
    };
    Ok((facts, record.doc_type))
}

/// The successor each numbered record of the directory holding `path`
/// names, by number. A record that cannot be listed or read is left out of
/// the map, so an unreadable file never blocks a supersede by itself.
fn successors_beside(store: &dyn DocStore, docs_dir: &Path, path: &Path) -> BTreeMap<i32, i32> {
    let mut links = BTreeMap::new();
    for other in store.list(docs_dir).unwrap_or_default() {
        if other.parent() != path.parent() {
            continue;
        }
        let Ok(text) = store.read(&other) else {
            continue;
        };
        let record = extract_record(&other, &text);
        if let (Some(number), Some(successor)) = (record.number, successor_number(&record)) {
            links.insert(number, successor);
        }
    }
    links
}

/// The record number a record's `superseded_by` names, when it is one.
pub(crate) fn successor_number(record: &ExtractedRecord) -> Option<i32> {
    record.superseded_by.as_deref()?.trim().parse().ok()
}

/// The cycles of a successor map (record number to successor number): each
/// is the sorted numbers of the records that name one another in a loop,
/// reported once however many records it holds. A chain that ends at a record
/// with no successor, or at a number with no record, is not a cycle.
pub(crate) fn supersede_cycles(successor_of: &BTreeMap<i32, i32>) -> Vec<Vec<i32>> {
    let mut seen = BTreeSet::new();
    let mut cycles = Vec::new();
    for &start in successor_of.keys() {
        let mut walked = Vec::new();
        let mut at = start;
        while !seen.contains(&at) {
            if let Some(from) = walked.iter().position(|&n| n == at) {
                let mut members: Vec<i32> = walked[from..].to_vec();
                members.sort_unstable();
                cycles.push(members);
                break;
            }
            walked.push(at);
            let Some(&next) = successor_of.get(&at) else {
                break;
            };
            at = next;
        }
        seen.extend(walked);
    }
    cycles
}

/// Refuses, before any write, a supersede of the record at `old` by the one
/// at `new` that the table does not allow. The message names the rule broken
/// and the records involved.
pub(crate) fn refuse_supersede_at(
    store: &dyn DocStore,
    docs_dir: &Path,
    old: &Path,
    new: &Path,
) -> Result<(), String> {
    let (old_facts, doc_type) = facts_at(store, old)?;
    let (new_facts, _) = facts_at(store, new)?;
    let Some(spec) = spec_for_frontmatter(&doc_type) else {
        return Ok(());
    };
    let links = successors_beside(store, docs_dir, old);
    refuse_supersede(spec, &old_facts, &new_facts, &links)
        .map_err(|refusal| format!("{}: {refusal}", old.display()))
}

#[cfg(test)]
mod tests;
