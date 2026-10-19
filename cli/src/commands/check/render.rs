//! Text rendering of the `check` report: violations first, advisories
//! collapsed to one summary line per kind, and the full advisory list only on
//! request, so a bundle with legacy debt keeps its violations readable.

use crate::output::{self, ColorMode, Style};
use living_docs_core::check::{Advisory, Report};
use std::collections::BTreeMap;
use std::path::Path;

const NAMED_RECORDS: usize = 5;

pub(super) fn render_text(report: &Report, color: ColorMode, list_advisories: bool) {
    println!("Living Docs lint — bundle: {}", report.bundle);
    println!();
    for violation in &report.violations {
        let line = finding_line(report, &violation.file, &violation.message);
        println!("{}", output::paint(color, Style::Red, &line));
    }
    if !report.advisories.is_empty() {
        render_advisories(report, color, list_advisories);
    }
    println!();
    render_verdict(report, color);
}

fn render_advisories(report: &Report, color: ColorMode, list_advisories: bool) {
    if !report.violations.is_empty() {
        println!();
    }
    for line in summary_lines(&report.advisories) {
        println!("{}", output::paint(color, Style::Yellow, &line));
    }
    if !list_advisories {
        println!("  run with --advisories to list each one");
        return;
    }
    println!();
    for advisory in &report.advisories {
        let line = finding_line(report, &advisory.file, &advisory.message);
        println!("{}", output::paint(color, Style::Yellow, &line));
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

/// One line per advisory kind, the largest group first and ties by kind name.
fn summary_lines(advisories: &[Advisory]) -> Vec<String> {
    let mut groups: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for advisory in advisories {
        groups
            .entry(advisory.kind.as_str())
            .or_default()
            .push(record_label(&advisory.file));
    }
    let mut groups: Vec<_> = groups.into_iter().collect();
    groups.sort_by_key(|(_, labels)| std::cmp::Reverse(labels.len()));
    groups
        .into_iter()
        .map(|(kind, labels)| summary_line(kind, &labels))
        .collect()
}

fn summary_line(kind: &str, labels: &[String]) -> String {
    let noun = if labels.len() == 1 {
        "advisory"
    } else {
        "advisories"
    };
    let named = labels[..labels.len().min(NAMED_RECORDS)].join(", ");
    let rest = labels.len().saturating_sub(NAMED_RECORDS);
    let overflow = if rest > 0 {
        format!(" (+{rest})")
    } else {
        String::new()
    };
    format!(
        "  {} {} {noun}: {named}{overflow}",
        labels.len(),
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
        println!("{}", output::paint(color, Style::Green, &line));
    } else {
        let line = format!(
            "FAIL — {} violation(s) across {} docs.",
            report.violations.len(),
            report.docs
        );
        println!("{}", output::paint(color, Style::Red, &line));
    }
}
