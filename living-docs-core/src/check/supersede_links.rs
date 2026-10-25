//! One-way supersede links: a record that names another in `supersedes` or
//! `superseded_by` while that other record does not name it back. The pair
//! is read from both sides, so a half-written replacement chain is caught at
//! the record that carries the unanswered link.

use super::records::for_each_readable_record;
use super::Reporter;
use crate::record::{extract_record, ExtractedRecord};
use crate::store::DocStore;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

type Entry = (PathBuf, ExtractedRecord);

/// Records keyed by `(doc_type, number)`, the pairing the supersede lineage
/// uses.
struct Records<'a> {
    bundle: &'a Path,
    by_key: BTreeMap<(&'a str, i32), Vec<&'a Entry>>,
}

impl<'a> Records<'a> {
    fn new(bundle: &'a Path, entries: &'a [Entry]) -> Self {
        let mut by_key: BTreeMap<(&str, i32), Vec<&Entry>> = BTreeMap::new();
        for entry in entries {
            if let Some(number) = entry.1.number {
                by_key
                    .entry((entry.1.doc_type.as_str(), number))
                    .or_default()
                    .push(entry);
            }
        }
        Self { bundle, by_key }
    }

    /// The one record of `doc_type` that `link` names, `None` when `link` is
    /// not a number or no record, or more than one, carries that number.
    fn target(&self, doc_type: &str, link: &str) -> Option<&'a Entry> {
        let number = link.trim().parse::<i32>().ok()?;
        match self.by_key.get(&(doc_type, number))?.as_slice() {
            [only] => Some(*only),
            _ => None,
        }
    }

    fn display(&self, path: &Path) -> String {
        path.strip_prefix(self.bundle)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}

/// Reports, on the carrying record, each `supersedes` entry whose record does
/// not name the carrier in `superseded_by`, and each `superseded_by` whose
/// record does not list the carrier in `supersedes`. Numbers compare as
/// integers; a link that is not a number, names no record of the carrier's
/// type, or names a number two records share is left to the other checks.
pub(crate) fn check_supersede_links(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let mut entries: Vec<Entry> = Vec::new();
    for_each_readable_record(store, all_md, |path, contents| {
        entries.push((path.to_path_buf(), extract_record(path, &contents)));
    });
    let records = Records::new(bundle, &entries);
    for entry in &entries {
        check_carrier(&records, entry, reporter);
    }
}

fn check_carrier(records: &Records, entry: &Entry, reporter: &mut Reporter) {
    if entry.1.number.is_some() {
        check_supersedes(records, entry, reporter);
        check_superseded_by(records, entry, reporter);
    }
}

fn check_supersedes(records: &Records, (path, record): &Entry, reporter: &mut Reporter) {
    for link in record.supersedes.iter().flatten() {
        let Some((target_path, target)) = records.target(&record.doc_type, link) else {
            continue;
        };
        let answered = target.superseded_by.as_deref().and_then(parse_number) == record.number;
        if !answered {
            report(records, reporter, path, "supersedes", link, target_path);
        }
    }
}

fn check_superseded_by(records: &Records, (path, record): &Entry, reporter: &mut Reporter) {
    let Some(link) = record.superseded_by.as_deref() else {
        return;
    };
    let Some((target_path, target)) = records.target(&record.doc_type, link) else {
        return;
    };
    let mut named = target.supersedes.iter().flatten().map(|s| parse_number(s));
    if !named.any(|n| n == record.number) {
        report(records, reporter, path, "superseded_by", link, target_path);
    }
}

fn parse_number(text: &str) -> Option<i32> {
    text.trim().parse().ok()
}

fn report(
    records: &Records,
    reporter: &mut Reporter,
    carrier: &Path,
    field: &str,
    link: &str,
    target: &Path,
) {
    let answer = if field == "supersedes" {
        "superseded_by"
    } else {
        "supersedes"
    };
    reporter.report(
        carrier,
        format!(
            "{field}: {link} names {}, which does not name {} in {answer}",
            records.display(target),
            records.display(carrier)
        ),
    );
}
