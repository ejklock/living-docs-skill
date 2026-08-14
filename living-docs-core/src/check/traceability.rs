//! Requirement-traceability check (ADR 0035): every `FR-N` / `NFR-N` a
//! non-Draft PRD defines must be cited by a BDR whose body links that PRD —
//! an advisory at `Accepted` (BDRs normally arrive after acceptance, so the
//! pre-commit gate must not block that window), a violation at `Implemented`
//! (an uncovered requirement there is drift). `Draft` and `Superseded` PRDs
//! are out of scope, as is a PRD defining no IDs, so ID-less legacy bundles
//! pass untouched.

use super::{file_name_str, records, Reporter};
use crate::frontmatter;
use crate::store::DocStore;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

enum Severity {
    Advisory,
    Violation,
}

pub(crate) fn check_requirement_traceability(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let (prds, bdrs) = collect_prds_and_bdrs(store, all_md);
    for (path, contents) in &prds {
        check_prd(path, contents, &bdrs, reporter);
    }
}

fn collect_prds_and_bdrs(
    store: &dyn DocStore,
    all_md: &[PathBuf],
) -> (Vec<(PathBuf, String)>, Vec<String>) {
    let mut prds = Vec::new();
    let mut bdrs = Vec::new();
    for f in all_md {
        if records::is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        match frontmatter::read_scalar_from_str(&contents, "type").as_deref() {
            Some("PRD") => prds.push((f.clone(), contents)),
            Some("BDR") => bdrs.push(contents),
            _ => {}
        }
    }
    (prds, bdrs)
}

fn severity_of(status: Option<&str>) -> Option<Severity> {
    match status? {
        "Accepted" => Some(Severity::Advisory),
        "Implemented" => Some(Severity::Violation),
        _ => None,
    }
}

fn check_prd(path: &Path, contents: &str, bdrs: &[String], reporter: &mut Reporter) {
    let status = frontmatter::read_scalar_from_str(contents, "status");
    let Some(severity) = severity_of(status.as_deref()) else {
        return;
    };
    let covered = ids_cited_by_linking_bdrs(&file_name_str(path), bdrs);
    for id in requirement_ids(contents) {
        if covered.contains(&id) {
            continue;
        }
        let message = format!(
            "TRACE {id} has no BDR coverage — no BDR links this PRD and cites the ID (ADR 0035)"
        );
        match severity {
            Severity::Advisory => reporter.advise(path, message),
            Severity::Violation => reporter.report(path, message),
        }
    }
}

fn ids_cited_by_linking_bdrs(prd_file_name: &str, bdrs: &[String]) -> BTreeSet<String> {
    let prd_link = format!("prd/{prd_file_name}");
    bdrs.iter()
        .filter(|bdr| bdr.contains(&prd_link))
        .flat_map(|bdr| requirement_ids(bdr))
        .collect()
}

/// Every `FR-N` / `NFR-N` token in `text`, boundary-checked on both sides so
/// `NFR-1` is never also counted as `FR-1`, `XFR-1` matches nothing, and a
/// digit-less `FR-` or letter-trailed `FR-1x` is not an ID.
pub(crate) fn requirement_ids(text: &str) -> BTreeSet<String> {
    text.match_indices("FR-")
        .filter_map(|(at, _)| requirement_id_at(text, at))
        .collect()
}

fn requirement_id_at(text: &str, at: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let start = if at > 0 && bytes[at - 1] == b'N' {
        at - 1
    } else {
        at
    };
    if start > 0 && bytes[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    let digits_from = at + "FR-".len();
    let digits = bytes[digits_from..]
        .iter()
        .take_while(|b| b.is_ascii_digit())
        .count();
    if digits == 0 {
        return None;
    }
    let end = digits_from + digits;
    if bytes.get(end).is_some_and(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    Some(text[start..end].to_string())
}

#[cfg(test)]
mod tests;
