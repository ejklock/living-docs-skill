//! `decision-table` advisory: a marked decision table under a `###` rule of a
//! domain record that leaves a combination of its conditions without a row,
//! gives one combination two different outcomes, hides a row behind earlier
//! rows under the `first` policy, names a value outside a condition's domain,
//! or has too many combinations to enumerate. A table is read only when the
//! line right before it is the marker. It never fails the gate.

mod parse;

use super::records::{for_each_readable_record, frontmatter_scalar};
use super::{kinds, Reporter};
use crate::doc_type;
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use parse::{parse_marker, parse_table, Cell, Policy, Scope, Table};
use std::path::{Path, PathBuf};

/// The most combinations a table may declare and still be enumerated.
const MAX_COMBINATIONS: usize = 4096;
/// The directory, and so the record type, whose records may hold the tables.
const DOMAIN_DIR: &str = "context";
/// How many uncovered combinations an advisory spells out.
const LISTED: usize = 3;

/// For each row, for each condition, the indexes into the condition's domain
/// the row covers.
type Coverage = Vec<Vec<Vec<usize>>>;

pub(crate) fn check_decision_table(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |file, contents| {
        if !is_domain_record(bundle, file, &contents) {
            return;
        }
        for (rule, finding) in findings_in(strip_frontmatter(&contents)) {
            reporter.advise(file, kinds::DECISION_TABLE, format!("{rule}: {finding}"));
        }
    });
}

/// Whether `path` is a record of the domain type, in the bundle's domain
/// directory.
fn is_domain_record(bundle: &Path, path: &Path, contents: &str) -> bool {
    let inside = path.strip_prefix(bundle).ok();
    let in_dir = inside
        .and_then(|rel| rel.components().next())
        .is_some_and(|part| part.as_os_str() == DOMAIN_DIR);
    let declared = frontmatter_scalar(contents, "type")
        .and_then(|declared| doc_type::spec_for_frontmatter(&declared));
    in_dir
        && declared.is_some_and(|spec| {
            doc_type::spec_for_dir(DOMAIN_DIR).is_some_and(|domain| std::ptr::eq(domain, spec))
        })
}

/// Every finding of every marked table in `body`, with the name of the `###`
/// rule it sits under; a marker outside such a rule, or inside a code fence,
/// is not read.
fn findings_in(body: &str) -> Vec<(String, String)> {
    let lines: Vec<&str> = body.lines().collect();
    let mut scope = Scope::default();
    let rules: Vec<Option<&str>> = lines.iter().map(|line| scope.rule_of(line)).collect();
    let mut found = Vec::new();
    let mut at = 0;
    while at < lines.len() {
        let marker = rules[at].zip(parse_marker(lines[at]));
        at += 1;
        if let Some((rule, marker)) = marker {
            let below = &lines[at..];
            let len = below
                .iter()
                .take_while(|l| l.trim_start().starts_with('|'))
                .count();
            at += len;
            let findings = judge(marker, &below[..len]);
            found.extend(findings.into_iter().map(|f| (rule.to_owned(), f)));
        }
    }
    found
}

/// The findings for one marker and the table lines that follow it.
fn judge(marker: Result<Policy, String>, table: &[&str]) -> Vec<String> {
    let policy = match marker {
        Ok(policy) => policy,
        Err(problem) => return vec![problem],
    };
    let parsed = parse_table(table).map(|parsed| prove(&parsed, policy));
    match (table.is_empty(), parsed) {
        (true, _) => vec!["the decision-table marker is not followed by a table".to_owned()],
        (false, parsed) => parsed.unwrap_or_else(|problem| vec![problem]),
    }
}

/// How many combinations the declared domains make.
fn combinations(table: &Table) -> usize {
    let sizes = table.conditions.iter().map(|c| c.domain.len());
    sizes.fold(1usize, usize::saturating_mul)
}

/// What each row covers; a value outside its condition's domain covers nothing.
fn covered(table: &Table) -> Coverage {
    table
        .rows
        .iter()
        .map(|row| {
            row.cells
                .iter()
                .zip(&table.conditions)
                .map(|(cell, condition)| match cell {
                    Cell::Any => (0..condition.domain.len()).collect(),
                    Cell::Values(values) => values
                        .iter()
                        .filter_map(|v| condition.domain.iter().position(|d| d == v))
                        .collect(),
                })
                .collect()
        })
        .collect()
}

/// One finding per row and condition that names values outside the domain.
fn outside_domain(table: &Table) -> Vec<String> {
    let mut found = Vec::new();
    for (number, row) in table.rows.iter().enumerate() {
        for (cell, condition) in row.cells.iter().zip(&table.conditions) {
            let Cell::Values(values) = cell else { continue };
            let unknown: Vec<String> = values
                .iter()
                .filter(|v| !condition.domain.contains(v))
                .map(|v| format!("'{v}'"))
                .collect();
            if !unknown.is_empty() {
                found.push(format!(
                    "row {} names {}, outside the values of {} ({})",
                    number + 1,
                    unknown.join(", "),
                    condition.name,
                    condition.domain.join(", ")
                ));
            }
        }
    }
    found
}

fn prove(table: &Table, policy: Policy) -> Vec<String> {
    let mut found = outside_domain(table);
    if combinations(table) > MAX_COMBINATIONS {
        found.push(format!(
            "too large to prove: more than {MAX_COMBINATIONS} combinations"
        ));
        return found;
    }
    let covers = covered(table);
    let (missing, used) = enumerate(table, &covers);
    found.extend(missing_finding(table, &missing));
    match policy {
        Policy::Unique => found.extend(contradictions(table, &covers)),
        Policy::First => found.extend(shadowed(&covers, &used)),
    }
    found
}

/// The condition values of combination number `n`, the last condition varying
/// fastest.
fn combination(table: &Table, mut n: usize) -> Vec<usize> {
    let mut values = vec![0; table.conditions.len()];
    for (slot, condition) in values.iter_mut().zip(&table.conditions).rev() {
        *slot = n % condition.domain.len();
        n /= condition.domain.len();
    }
    values
}

/// The combinations no row covers, and which rows are the first to match some
/// combination.
fn enumerate(table: &Table, covers: &Coverage) -> (Vec<Vec<usize>>, Vec<bool>) {
    let mut missing = Vec::new();
    let mut used = vec![false; covers.len()];
    for n in 0..combinations(table) {
        let values = combination(table, n);
        let first = covers
            .iter()
            .position(|row| row.iter().zip(&values).all(|(set, v)| set.contains(v)));
        match first {
            Some(row) => used[row] = true,
            None => missing.push(values),
        }
    }
    (missing, used)
}

/// `name=value` for each condition, in column order.
fn describe(table: &Table, values: &[usize]) -> String {
    let named = table.conditions.iter().zip(values);
    let named: Vec<String> = named
        .map(|(c, v)| format!("{}={}", c.name, c.domain[*v]))
        .collect();
    named.join(", ")
}

fn missing_finding(table: &Table, missing: &[Vec<usize>]) -> Option<String> {
    let listed = missing
        .iter()
        .take(LISTED)
        .map(|values| describe(table, values));
    let listed = listed.collect::<Vec<_>>().join("; ");
    match missing.len() {
        0 => None,
        1 => Some(format!("no row covers {listed}")),
        n => Some(format!(
            "no row covers {n} combinations, among them {listed}"
        )),
    }
}

/// One finding per pair of rows that cover a common combination with
/// different outcomes.
fn contradictions(table: &Table, covers: &Coverage) -> Vec<String> {
    let mut found = Vec::new();
    for (i, first) in covers.iter().enumerate() {
        for (offset, second) in covers[i + 1..].iter().enumerate() {
            let (a, b) = (&table.rows[i], &table.rows[i + 1 + offset]);
            let common: Option<Vec<usize>> = first
                .iter()
                .zip(second)
                .map(|(x, y)| x.iter().copied().find(|v| y.contains(v)))
                .collect();
            if let Some(values) = common.filter(|_| a.outcome != b.outcome) {
                found.push(format!(
                    "rows {} and {} both cover {} but give different outcomes ('{}' and '{}')",
                    i + 1,
                    i + 2 + offset,
                    describe(table, &values),
                    a.outcome,
                    b.outcome
                ));
            }
        }
    }
    found
}

/// One finding per row that covers something yet is never the first to match.
fn shadowed(covers: &Coverage, used: &[bool]) -> Vec<String> {
    covers
        .iter()
        .zip(used)
        .enumerate()
        .filter(|(_, (row, used))| !**used && row.iter().all(|set| !set.is_empty()))
        .map(|(i, _)| format!("row {} is fully shadowed by earlier rows", i + 1))
        .collect()
}

#[cfg(test)]
mod tests;
