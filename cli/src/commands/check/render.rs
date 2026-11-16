//! Text rendering of the `check` report: violations first, advisories
//! collapsed to one summary line per kind, and the full advisory list only on
//! request, so a bundle with legacy debt keeps its violations readable.

use crate::output::{self, ColorMode, Style};
use living_docs_core::check::{Advisory, Report};
use std::collections::BTreeMap;
use std::path::Path;

const NAMED_RECORDS: usize = 5;

pub(super) fn render_text(report: &Report, color: ColorMode, list_advisories: bool) {
    output::outln!("Living Docs lint — bundle: {}", report.bundle);
    output::outln!();
    for violation in &report.violations {
        let line = finding_line(report, &violation.file, &violation.message);
        output::outln!("{}", output::paint(color, Style::Red, &line));
    }
    if !report.advisories.is_empty() {
        render_advisories(report, color, list_advisories);
    }
    output::outln!();
    render_verdict(report, color);
}

fn render_advisories(report: &Report, color: ColorMode, list_advisories: bool) {
    if !report.violations.is_empty() {
        output::outln!();
    }
    for line in summary_lines(&report.advisories) {
        output::outln!("{}", output::paint(color, Style::Yellow, &line));
    }
    if !list_advisories {
        output::outln!("  run with --advisories to list each one");
        return;
    }
    output::outln!();
    for advisory in &report.advisories {
        let line = finding_line(report, &advisory.file, &advisory.message);
        output::outln!("{}", output::paint(color, Style::Yellow, &line));
    }
}

fn finding_line(report: &Report, file: &str, message: &str) -> String {
    format!("  {:<44} {}", relative_to_bundle(report, file), message)
}

/// A finding's path as the reader would type it from the bundle root; a path
/// outside the bundle is left untouched rather than guessed at.
fn relative_to_bundle(report: &Report, file: &str) -> String {
    Path::new(file)
        .strip_prefix(&report.bundle)
        .map_or_else(|_| file.to_owned(), |rel| rel.display().to_string())
}

/// One advisory kind's findings: how many there are and the distinct records
/// they fall on, in the order the records first appear.
#[derive(Default)]
struct KindGroup {
    advisories: usize,
    records: Vec<String>,
}

/// One line per advisory kind, the largest group first and ties by kind name.
fn summary_lines(advisories: &[Advisory]) -> Vec<String> {
    let mut groups: BTreeMap<&str, KindGroup> = BTreeMap::new();
    for advisory in advisories {
        let group = groups.entry(advisory.kind.as_str()).or_default();
        group.advisories += 1;
        let label = record_label(&advisory.file);
        if !group.records.contains(&label) {
            group.records.push(label);
        }
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, group)| std::cmp::Reverse(group.advisories));
    groups
        .into_iter()
        .map(|(kind, group)| summary_line(kind, &group))
        .collect()
}

fn summary_line(kind: &str, group: &KindGroup) -> String {
    let noun = if group.advisories == 1 {
        "advisory"
    } else {
        "advisories"
    };
    let records = &group.records;
    let named = records[..records.len().min(NAMED_RECORDS)].join(", ");
    let rest = records.len().saturating_sub(NAMED_RECORDS);
    let overflow = if rest > 0 {
        format!(" (+{rest})")
    } else {
        String::new()
    };
    format!(
        "  {} {} {noun}: {named}{overflow}",
        group.advisories,
        kind.to_uppercase()
    )
}

/// The record's `NNNN` number when its file name starts with one, else the
/// file stem.
fn record_label(file: &str) -> String {
    let stem = Path::new(file)
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        stem
    } else {
        digits
    }
}

fn render_verdict(report: &Report, color: ColorMode) {
    if report.ok {
        let line = format!("OK — {} docs, no invariant violations.", report.docs);
        output::outln!("{}", output::paint(color, Style::Green, &line));
    } else {
        let line = format!(
            "FAIL — {} violation(s) across {} docs.",
            report.violations.len(),
            report.docs
        );
        output::outln!("{}", output::paint(color, Style::Red, &line));
    }
}
