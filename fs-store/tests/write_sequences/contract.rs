//! The contract a bundle keeps after every write step, in the order it is
//! judged.

use crate::ops;
use crate::snapshot::{self, Entry, Snapshot};
use fs_store::FsStore;
use living_docs_core::check;
use living_docs_core::commands::{effective, index};
use living_docs_core::doc_type::spec_for_frontmatter;
use living_docs_core::record::extract_record;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The part of the contract a bundle broke.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    RefusedStepChangedFile,
    IndexDiffersFromRegeneration,
    RetiredRecordWithoutCallout,
    UnmirroredSupersedeLink,
    NonCanonicalFrontmatter,
    DuplicateNumber,
    RecordNotListedInIndex,
    OtherViolation,
    StaleSuccessor,
    InForceWithItsSuccessor,
    InForceViewUnreadable,
    FmtThenIndexChangedBytes,
    SupersedeRewroteLink,
    SupersedeCycle,
    SupersedeEndsRetired,
    StatusChangedOnSuperseded,
}

/// A broken part and what named it.
#[derive(Debug)]
pub struct Breach {
    pub part: Part,
    pub detail: String,
}

fn breach(part: Part, detail: impl Into<String>) -> Result<(), Breach> {
    Err(Breach {
        part,
        detail: detail.into(),
    })
}

/// Judges the bundle after a step; `before` is its bytes ahead of the step
/// and `refused` tells whether the step was refused.
pub fn verify(bundle: &Path, before: &Snapshot, refused: bool) -> Result<(), Breach> {
    if refused {
        refused_step_left_bundle_alone(before, &snapshot::take(bundle))?;
    }
    judge_state(bundle).map(drop)
}

/// Judges only that a refused step left every file and directory as it was.
pub fn refused_step_left_bundle_alone(before: &Snapshot, after: &Snapshot) -> Result<(), Breach> {
    let changed = snapshot::changed(before, after);
    if changed.is_empty() {
        return Ok(());
    }
    breach(Part::RefusedStepChangedFile, format!("{changed:?}"))
}

/// One judgment of a bundle as it stands.
pub type StatePart = fn(&FsStore, &Path) -> Result<(), Breach>;

pub const IN_FORCE_WITHOUT_SUCCESSOR: usize = 1;
pub const SUCCESSOR_CHAINS_ARE_ACYCLIC: usize = 2;
pub const CANONICAL_AFTER_FMT_AND_INDEX: usize = 3;

/// The judgments of a bundle as it stands, each with its name, in the order
/// they run.
pub const STATE_PARTS: [(&str, StatePart); 4] = [
    ("check_report", check_report),
    ("in_force_without_successor", in_force_without_successor),
    ("successor_chains_are_acyclic", successor_chains_are_acyclic),
    (
        "canonical_after_fmt_and_index",
        canonical_after_fmt_and_index,
    ),
];

/// Judges the bundle as it stands, whatever step led to it, and returns the
/// names of the judgments it ran.
pub fn judge_state(bundle: &Path) -> Result<Vec<&'static str>, Breach> {
    judge_from(bundle, 0)
}

/// Judges the bundle with [`STATE_PARTS`] from index `first` on and returns
/// the names of the judgments it ran.
pub fn judge_from(bundle: &Path, first: usize) -> Result<Vec<&'static str>, Breach> {
    let store = FsStore::rooted(bundle);
    let mut ran = Vec::new();
    for (name, part) in &STATE_PARTS[first..] {
        part(&store, bundle)?;
        ran.push(*name);
    }
    Ok(ran)
}

fn part_of(message: &str) -> Part {
    let parts = [
        (
            "index differs from its regeneration",
            Part::IndexDiffersFromRegeneration,
        ),
        ("callout", Part::RetiredRecordWithoutCallout),
        ("does not name", Part::UnmirroredSupersedeLink),
        ("non-canonical", Part::NonCanonicalFrontmatter),
        ("share the number", Part::DuplicateNumber),
        ("not listed in", Part::RecordNotListedInIndex),
        ("status is not Superseded", Part::StaleSuccessor),
    ];
    parts
        .iter()
        .find(|(needle, _)| message.contains(needle))
        .map_or(Part::OtherViolation, |(_, part)| *part)
}

fn check_report(store: &FsStore, bundle: &Path) -> Result<(), Breach> {
    let report = check::compile(store, bundle, false).expect("the bundle is a directory");
    if let Some(violation) = report.violations.first() {
        let detail = format!("{}: {}", violation.file, violation.message);
        return breach(part_of(&violation.message), detail);
    }
    Ok(())
}

fn record_texts(bundle: &Path) -> Vec<(PathBuf, String)> {
    texts_in(&snapshot::take(bundle))
}

fn texts_in(files: &Snapshot) -> Vec<(PathBuf, String)> {
    files
        .iter()
        .filter(|(path, _)| path.file_name().is_some_and(|name| name != "index.md"))
        .filter_map(|(path, entry)| match entry {
            Entry::File(bytes) => String::from_utf8(bytes.clone())
                .ok()
                .map(|t| (path.clone(), t)),
            Entry::Dir => None,
        })
        .filter(|(_, text)| text.starts_with("---"))
        .collect()
}

fn in_force_labels(store: &FsStore, bundle: &Path) -> Result<BTreeSet<String>, Breach> {
    let options = effective::Options {
        topic: None,
        full: false,
        contract: false,
        all: false,
        dependents: None,
    };
    let reading = effective::read(store, bundle, &options, false);
    let labels: BTreeSet<String> = reading
        .output
        .lines()
        .filter_map(|line| line.strip_prefix("- ["))
        .filter_map(|rest| rest.split_once(']'))
        .map(|(label, _)| label.to_string())
        .collect();
    let records = record_texts(bundle).len();
    if labels.len() != reading.matched || reading.matched + reading.withheld != records {
        let detail = format!(
            "the view lists {} of {} in-force records with {} withheld, for {records} records:\n{}",
            labels.len(),
            reading.matched,
            reading.withheld,
            reading.output
        );
        return breach(Part::InForceViewUnreadable, detail).map(|()| labels);
    }
    Ok(labels)
}

/// Judges that no record is in force together with its own successor.
fn in_force_without_successor(store: &FsStore, bundle: &Path) -> Result<(), Breach> {
    let in_force = in_force_labels(store, bundle)?;
    for (path, text) in record_texts(bundle) {
        let record = extract_record(&path, &text);
        let (Some(number), Some(successor)) = (record.number, record.superseded_by.as_deref())
        else {
            continue;
        };
        let own = format!("{} {number:04}", record.doc_type);
        let theirs = format!("{} {successor}", record.doc_type);
        if in_force.contains(&own) && in_force.contains(&theirs) {
            return breach(Part::InForceWithItsSuccessor, format!("{own} and {theirs}"));
        }
    }
    Ok(())
}

/// Judges that running fmt and then index changes no byte.
fn canonical_after_fmt_and_index(store: &FsStore, bundle: &Path) -> Result<(), Breach> {
    let before = snapshot::take(bundle);
    let outcome = ops::format_bundle(store, bundle)
        .and_then(|()| index::write(store, bundle, None).map(drop));
    let changed = snapshot::changed(&before, &snapshot::take(bundle));
    match (outcome, changed.is_empty()) {
        (Ok(()), true) => Ok(()),
        (Ok(()), false) => breach(Part::FmtThenIndexChangedBytes, format!("{changed:?}")),
        (Err(message), _) => breach(
            Part::FmtThenIndexChangedBytes,
            format!("refused: {message}"),
        ),
    }
}

/// The successor each record of `texts` names, by record label.
fn successor_links(texts: &[(PathBuf, String)]) -> BTreeMap<String, String> {
    let mut links = BTreeMap::new();
    for (path, text) in texts {
        let record = extract_record(path, text);
        if let (Some(number), Some(successor)) = (record.number, record.superseded_by.as_deref()) {
            let own = format!("{} {number:04}", record.doc_type);
            links.insert(own, format!("{} {successor}", record.doc_type));
        }
    }
    links
}

fn is_retired(doc_type: &str, status: &str) -> bool {
    spec_for_frontmatter(doc_type).is_some_and(|spec| spec.is_retired(status))
}

/// The labels of the records whose status retires them.
fn retired_labels(texts: &[(PathBuf, String)]) -> BTreeSet<String> {
    texts
        .iter()
        .map(|(path, text)| extract_record(path, text))
        .filter(|r| {
            r.status
                .as_deref()
                .is_some_and(|st| is_retired(&r.doc_type, st))
        })
        .filter_map(|record| Some(format!("{} {:04}", record.doc_type, record.number?)))
        .collect()
}

/// The record where following successor links from `start` closes a loop,
/// when it does.
fn loop_closing(links: &BTreeMap<String, String>, start: &str) -> Option<String> {
    let mut seen = vec![start];
    while let Some(next) = links.get(*seen.last()?) {
        if seen.contains(&next.as_str()) {
            return Some(next.clone());
        }
        seen.push(next);
    }
    None
}

/// Judges that every supersede chain is acyclic; a chain may end in a
/// retired record, which `write set status Deprecated` reaches.
fn successor_chains_are_acyclic(_: &FsStore, bundle: &Path) -> Result<(), Breach> {
    let links = successor_links(&record_texts(bundle));
    for start in links.keys() {
        if let Some(closing) = loop_closing(&links, start) {
            breach(Part::SupersedeCycle, format!("{start} loops at {closing}"))?;
        }
    }
    Ok(())
}

/// Judges an accepted status change by the step alone: the record it named
/// had no successor before it, since a superseded record refuses every status.
pub fn status_step_is_legal(before: &Snapshot, doc_type: &str, number: u32) -> Result<(), Breach> {
    let label = format!("{doc_type} {number:04}");
    if successor_links(&texts_in(before)).contains_key(&label) {
        return breach(
            Part::StatusChangedOnSuperseded,
            format!("{label} took a status"),
        );
    }
    Ok(())
}

/// Judges an accepted `write supersede` by the step alone: no existing
/// successor link changed, the chain is acyclic and a link added ends live.
pub fn supersede_step_is_legal(before: &Snapshot, bundle: &Path, new: &str) -> Result<(), Breach> {
    let earlier = successor_links(&texts_in(before));
    let now = successor_links(&record_texts(bundle));
    if let Some((record, successor)) = earlier.iter().find(|(r, s)| now.get(*r) != Some(s)) {
        let detail = format!("{record} named {successor}, now {:?}", now.get(record));
        return breach(Part::SupersedeRewroteLink, detail);
    }
    successor_chains_are_acyclic(&FsStore::rooted(bundle), bundle)?;
    let label = new.replace('/', " ");
    let retired = retired_labels(&record_texts(bundle));
    let repeat = earlier.len() == now.len();
    if !repeat && retired.iter().any(|own| own.eq_ignore_ascii_case(&label)) {
        return breach(Part::SupersedeEndsRetired, format!("{new} is retired"));
    }
    Ok(())
}
