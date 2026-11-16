//! Issue dependencies declared in `blocked_by`. Numeric items are resolved:
//! each should name an issue of the bundle, no issue should wait on itself,
//! and no issues should wait on one another in a cycle. Every finding,
//! including a non-numeric item, is advice.

use super::records::for_each_readable_record;
use super::{kinds, Reporter};
use crate::frontmatter::scalar_to_string;
use crate::record::{extract_record, TailValue};
use crate::store::DocStore;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

type Graph = BTreeMap<u32, BTreeSet<u32>>;
type Paths<'a> = BTreeMap<u32, &'a Path>;

/// An issue record reduced to what the pass reads: where it lives, its
/// filename number, and its `blocked_by` items as written.
struct Issue {
    path: PathBuf,
    number: Option<u32>,
    items: Vec<String>,
}

/// Advises, on the issue that carries it and once per item, on a `blocked_by`
/// item that names no issue of the bundle, on an issue that lists itself, and
/// on an item that is not a decimal number (which never takes part in a
/// cycle); advises once per cycle of issues waiting on one another, anchored
/// on every member so a scope holding any one of them keeps it. Every finding
/// is advice and never fails the check. Items compare
/// as integers (`12` and `0012` are one issue) and a blocker in any status
/// resolves. Resolution reads every issue of `all_md`, whatever the scope;
/// records of any other type are never judged.
pub(crate) fn check_dependencies(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let issues = read_issues(store, all_md);
    let paths: Paths = issues
        .iter()
        .rev()
        .filter_map(|issue| Some((issue.number?, issue.path.as_path())))
        .collect();
    for issue in &issues {
        check_items(issue, &paths, reporter);
    }
    for cycle in cycles(&edges(&issues, &paths)) {
        let members: Vec<PathBuf> = cycle.iter().map(|n| paths[n].to_path_buf()).collect();
        let names: Vec<String> = members.iter().map(|p| relative(bundle, p)).collect();
        let message = format!(
            "issues wait on one another in a cycle through blocked_by: {}",
            names.join(", ")
        );
        reporter.advise_group(&members, kinds::BLOCKED_BY, message);
    }
}

fn relative(bundle: &Path, path: &Path) -> String {
    path.strip_prefix(bundle)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn read_issues(store: &dyn DocStore, all_md: &[PathBuf]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for_each_readable_record(store, all_md, |path, contents| {
        let record = extract_record(path, &contents);
        if record.doc_type == "Issue" {
            issues.push(Issue {
                path: path.to_path_buf(),
                number: record.number.and_then(|n| u32::try_from(n).ok()),
                items: blocked_by_items(&record.frontmatter_tail),
            });
        }
    });
    issues
}

/// The `blocked_by` entries as text: a flow list, or a scalar read as a
/// comma-separated list.
fn blocked_by_items(tail: &[(String, TailValue)]) -> Vec<String> {
    let Some((_, value)) = tail.iter().find(|(key, _)| key == "blocked_by") else {
        return Vec::new();
    };
    match value {
        TailValue::Scalar(text) => text
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(str::to_string)
            .collect(),
        TailValue::Sequence(items) => items.clone(),
        TailValue::Typed(values) => values
            .iter()
            .map(|v| scalar_to_string(v).unwrap_or_default())
            .collect(),
    }
}

/// Whether `item` is a decimal number: ASCII digits only. Such an item is
/// validated even when it overflows the issue number range.
fn is_decimal(item: &str) -> bool {
    let item = item.trim();
    !item.is_empty() && item.bytes().all(|b| b.is_ascii_digit())
}

/// The issue number a decimal `item` names; `None` for any other item and for
/// a decimal beyond the integer range, which names no issue.
fn item_number(item: &str) -> Option<u32> {
    if is_decimal(item) {
        item.trim().parse().ok()
    } else {
        None
    }
}

fn check_items(issue: &Issue, paths: &Paths, reporter: &mut Reporter) {
    for item in &issue.items {
        let advice = if !is_decimal(item) {
            format!("blocked_by: '{item}' is not an issue number, so it is not validated")
        } else {
            match item_number(item) {
                Some(n) if Some(n) == issue.number => {
                    format!("blocked_by: {item} lists the issue itself")
                }
                Some(n) if paths.contains_key(&n) => continue,
                _ => format!("blocked_by: {item} names no issue of the bundle"),
            }
        };
        reporter.advise(&issue.path, kinds::BLOCKED_BY, advice);
    }
}

/// The wait-for graph over resolvable issue numbers, self-references left out
/// (they are reported on their own).
fn edges(issues: &[Issue], paths: &Paths) -> Graph {
    let mut graph = Graph::new();
    for issue in issues {
        let Some(from) = issue.number else { continue };
        let targets = issue
            .items
            .iter()
            .filter_map(|item| item_number(item))
            .filter(|n| *n != from && paths.contains_key(n));
        graph.entry(from).or_default().extend(targets);
    }
    graph
}

/// Every strongly connected set of two or more issues, members ascending,
/// sets ordered by their first member.
fn cycles(graph: &Graph) -> Vec<Vec<u32>> {
    let reach: Graph = graph
        .keys()
        .map(|&start| (start, reachable(graph, start)))
        .collect();
    let mut found = Vec::new();
    for (&start, from_start) in &reach {
        let members: Vec<u32> = from_start
            .iter()
            .copied()
            .filter(|n| reach.get(n).is_some_and(|back| back.contains(&start)))
            .collect();
        if from_start.contains(&start) && members.iter().all(|&m| m >= start) {
            found.push(members);
        }
    }
    found
}

/// The issues `start` waits on, directly or through others.
fn reachable(graph: &Graph, start: u32) -> BTreeSet<u32> {
    let mut seen = BTreeSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        for &next in graph.get(&node).into_iter().flatten() {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    seen
}
