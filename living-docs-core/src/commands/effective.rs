//! `living-docs read` (the module keeps its original name `effective`
//! internally): the agent-facing read of the bundle — active records only
//! (superseded/deprecated withheld), supersede chains collapsed to the head
//! with a one-line lineage, grouped by kind then number. `--topic` restricts
//! to records where every term starts a word of the title, description or
//! body (case- and accent-insensitive) and ranks them by term frequency.
//! `--full` prints bodies instead of the one-line index, and `--contract` the
//! same minus detail-tier sections. Derived, never committed: the records
//! stay the SSOT.

use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use crate::{doc_type, sections};
pub use dependents::{resolve_target, Dependents};
use json::json_of;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod dependents;
mod json;
mod render;
mod topic;

/// Everything `effective` needs from the CLI front.
pub struct Options {
    pub topic: Option<String>,
    pub full: bool,
    pub contract: bool,
    /// Narrows the view to the dependents of one record; excludes `topic`.
    pub dependents: Option<Dependents>,
}

/// One record that survived selection, flattened to what the renderer needs.
pub(crate) struct View {
    pub doc_type: String,
    pub number: Option<i32>,
    pub title: String,
    pub description: String,
    pub body: String,
    pub lineage: Option<String>,
    /// Link distance to the queried record; set only under `Dependents::Impact`.
    pub distance: Option<usize>,
}

/// A read record paired with its `bundle`-prefixed path.
type Entry = (PathBuf, ExtractedRecord);

pub fn run(store: &dyn DocStore, bundle: &Path, options: &Options) -> ExitCode {
    print!("{}", compile(store, bundle, options));
    ExitCode::SUCCESS
}

/// Selects and orders the in-force view's records — the pure core shared by
/// [`compile`] (text) and [`compile_json`] (JSON) — returning the
/// withheld count and the surviving [`View`]s in reading order.
pub(crate) fn collect(
    store: &dyn DocStore,
    bundle: &Path,
    options: &Options,
) -> (usize, Vec<View>) {
    let records = read_records(store, &store.list(bundle).unwrap_or_default());
    let withheld = records
        .iter()
        .filter(|(_, record)| !is_in_force(record.status.as_deref()))
        .count();
    let ranked = rank_in_force(&records, options.topic.as_deref());
    let selected = match &options.dependents {
        Some(query) => dependents::narrow(&records, bundle, query, ranked),
        None => ranked.into_iter().map(|index| (index, None)).collect(),
    };
    let by_key = records_by_key(&records);
    let views = selected
        .into_iter()
        .map(|(index, distance)| View {
            distance,
            ..view_of(&records[index].1, &by_key, options.contract)
        })
        .collect();
    (withheld, views)
}

/// The indexes of the in-force records that match `topic`, best match first,
/// then by reading order.
fn rank_in_force(records: &[Entry], topic: Option<&str>) -> Vec<usize> {
    let terms = topic.map(topic::terms).unwrap_or_default();
    let mut active: Vec<(usize, topic::Folded)> = records
        .iter()
        .enumerate()
        .filter(|(_, (_, record))| is_in_force(record.status.as_deref()))
        .map(|(index, (_, record))| (index, topic::fold_for(record, &terms)))
        .filter(|(_, folded)| topic::matches(folded, &terms))
        .collect();
    active.sort_by_cached_key(|(index, folded)| {
        (
            std::cmp::Reverse(topic::score(folded, &terms)),
            base_rank(&records[*index].1),
        )
    });
    active.into_iter().map(|(index, _)| index).collect()
}

/// A compiled view plus the counts a front needs to explain an empty result
/// without a second pass over the bundle.
pub struct Reading {
    pub output: String,
    pub matched: usize,
    pub withheld: usize,
}

/// Compiles the effective view as a string — the pure core, so tests assert on
/// the compiled text without capturing stdout.
pub fn compile(store: &dyn DocStore, bundle: &Path, options: &Options) -> String {
    read(store, bundle, options, false).output
}

/// Reads the bundle once and renders it as text, or as JSON when `json` is set.
pub fn read(store: &dyn DocStore, bundle: &Path, options: &Options, json: bool) -> Reading {
    let (withheld, views) = collect(store, bundle, options);
    let matched = views.len();
    let output = if json {
        json_of(withheld, views, options)
    } else {
        render::render(&views, options.full || options.contract, withheld)
    };
    Reading {
        output,
        matched,
        withheld,
    }
}

/// Minified-JSON counterpart of [`compile`]:
/// `{"withheld":N,"records":[{"type","number","title","description",
/// "lineage","body"?,"distance"?}]}` — `body` appears under `options.full`
/// and under `options.contract`, the latter without detail-tier sections;
/// `distance` appears only under `Dependents::Impact`.
pub fn compile_json(store: &dyn DocStore, bundle: &Path, options: &Options) -> String {
    read(store, bundle, options, true).output
}

fn read_records(store: &dyn DocStore, all_md: &[PathBuf]) -> Vec<(PathBuf, ExtractedRecord)> {
    all_md
        .iter()
        .filter(|path| !is_reserved(path))
        .filter_map(|path| {
            let contents = store.read(path).ok()?;
            let record = record::extract_record(path, &contents);
            (!record.doc_type.is_empty()).then(|| (path.clone(), record))
        })
        .collect()
}

fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md") | Some("log.md")
    )
}

fn is_in_force(status: Option<&str>) -> bool {
    !matches!(
        status.map(|s| s.to_ascii_lowercase()).as_deref(),
        Some("superseded") | Some("deprecated")
    )
}

fn view_of(record: &ExtractedRecord, by_key: &RecordsByKey, contract: bool) -> View {
    let body = if contract {
        let schema =
            doc_type::spec_for_frontmatter(&record.doc_type).map_or(&[][..], |s| s.sections);
        sections::contract_body(&record.body, schema)
    } else {
        record.body.clone()
    };
    View {
        doc_type: record.doc_type.clone(),
        number: record.number,
        title: record.title.clone(),
        description: record.description.clone(),
        body,
        lineage: lineage_of(record, by_key),
        distance: None,
    }
}

/// The lineage line for a head record. A single chain `0131 → 0133 → head`
/// reads `supersedes 0131 via 0133`; a lone ancestor reads `supersedes 0133`;
/// a record replacing several lists every earliest ancestor comma-separated
/// (`supersedes 0001, 0002`), followed by ` via` the intermediate records
/// when there are any. `None` when the record supersedes nothing.
fn lineage_of(record: &ExtractedRecord, by_key: &RecordsByKey) -> Option<String> {
    let mut lineage = supersede_lineage(record, by_key);
    if lineage.earliest.is_empty() {
        lineage.earliest.extend(lineage.intermediate.pop());
    }
    if lineage.earliest.is_empty() {
        return None;
    }
    let mut line = format!("supersedes {}", lineage.earliest.join(", "));
    if !lineage.intermediate.is_empty() {
        line.push_str(&format!(" via {}", lineage.intermediate.join(", ")));
    }
    Some(line)
}

/// The ancestors of a head record, as zero-padded numbers in discovery order.
#[derive(Default)]
struct Lineage {
    /// Ancestors that replaced others in turn, direct-first.
    intermediate: Vec<String>,
    /// Ancestors with nothing further to follow.
    earliest: Vec<String>,
}

type RecordsByKey<'a> = BTreeMap<(String, i32), &'a ExtractedRecord>;

/// Indexes every numbered record, a later duplicate replacing an earlier one.
fn records_by_key(records: &[Entry]) -> RecordsByKey<'_> {
    records
        .iter()
        .filter_map(|(_, r)| Some(((r.doc_type.clone(), r.number?), r)))
        .collect()
}

/// Walks `supersedes` from `record` through the full record set (superseded
/// ancestors included). A cycle or missing link stops that branch.
fn supersede_lineage(record: &ExtractedRecord, by_key: &RecordsByKey) -> Lineage {
    let mut lineage = Lineage::default();
    let mut seen = Vec::new();
    walk_ancestors(record, by_key, &mut seen, &mut lineage);
    lineage
}

/// Visits each not-yet-seen ancestor named by `record.supersedes`, sorting it
/// into `lineage` by whether it has unseen ancestors of its own to follow.
fn walk_ancestors(
    record: &ExtractedRecord,
    by_key: &RecordsByKey,
    seen: &mut Vec<i32>,
    lineage: &mut Lineage,
) {
    let numbers = record.supersedes.iter().flatten();
    for number in numbers.filter_map(|s| s.trim().parse::<i32>().ok()) {
        if seen.contains(&number) {
            continue;
        }
        seen.push(number);
        let ancestor = by_key
            .get(&(record.doc_type.clone(), number))
            .filter(|a| has_ancestor(a));
        if let Some(ancestor) = ancestor {
            lineage.intermediate.push(format!("{number:04}"));
            walk_ancestors(ancestor, by_key, seen, lineage);
        } else {
            lineage.earliest.push(format!("{number:04}"));
        }
    }
}

/// Whether `record` names at least one ancestor by a readable number, so it
/// is an intermediate step rather than an earliest record.
fn has_ancestor(record: &ExtractedRecord) -> bool {
    record
        .supersedes
        .iter()
        .flatten()
        .any(|s| s.trim().parse::<i32>().is_ok())
}

/// Reading order: constitution and PRDs first, then ADRs, then everything
/// else, each by number. A grouping for orientation, not a relevance rank.
fn base_rank(record: &ExtractedRecord) -> (u8, i32) {
    let group = match record.doc_type.as_str() {
        "Constitution" => 0,
        "PRD" => 1,
        "ADR" => 2,
        _ => 3,
    };
    (group, record.number.unwrap_or(i32::MAX))
}

#[cfg(test)]
mod tests;
