//! Mechanical record liveness (ADR 0049, trimmed by ADR 0057): `check` emits a
//! `stale-proposed` advisory for an ADR still at its seed status (`Proposed`)
//! that links a terminal or superseded issue — the exit code never moves, same
//! posture as `SIZE` and `MOVED-SOURCE`. Currency has a sound oracle: the
//! linked issue's status.

use super::links::{link_destinations, resolve_destination};
use super::records::{frontmatter_scalar, is_reserved};
use super::{file_name_str, kinds, Reporter};
use crate::doc_type;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_liveness(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let bundle_str = bundle.to_string_lossy();
    for f in all_md {
        if is_stale_proposed(store, f, &bundle_str, all_md) {
            reporter.advise(
                f,
                kinds::LIVENESS,
                "LIVENESS stale-proposed: seed status but the linked issue is terminal — settle or supersede",
            );
        }
    }
}

/// True when `f` is an ADR still at its seed status (`Proposed`) that links an
/// issue whose status is terminal — the decision's work is done but the record
/// never left its birth state.
fn is_stale_proposed(store: &dyn DocStore, f: &Path, bundle: &str, all_md: &[PathBuf]) -> bool {
    if is_reserved(&file_name_str(f)) {
        return false;
    }
    let Ok(contents) = store.read(f) else {
        return false;
    };
    let Some(doc_type) = frontmatter_scalar(&contents, "type") else {
        return false;
    };
    if !is_liveness_type(&doc_type) {
        return false;
    }
    let Some(status) = frontmatter_scalar(&contents, "status") else {
        return false;
    };
    is_seed_status(&doc_type, &status) && links_terminal_issue(store, f, &contents, bundle, all_md)
}

/// Only ADR carries a liveness signal (ADR 0049): a decision record's currency
/// is checkable against its linked issue.
fn is_liveness_type(doc_type: &str) -> bool {
    doc_type == "ADR"
}

/// The seed status is `status_vocabulary[0]` — `Proposed` for ADR — the state
/// a record is born in and should leave once its work lands (registry-sourced,
/// ADR 0029).
fn is_seed_status(doc_type: &str, status: &str) -> bool {
    doc_type::spec_for_frontmatter(doc_type)
        .and_then(|spec| spec.status_vocabulary.first())
        .is_some_and(|seed| seed.eq_ignore_ascii_case(status))
}

/// True when `f` links a bundle-relative issue record whose status is terminal
/// for its type (`closed`/`done`) or `Superseded`.
fn links_terminal_issue(
    store: &dyn DocStore,
    f: &Path,
    contents: &str,
    bundle: &str,
    all_md: &[PathBuf],
) -> bool {
    let file_str = f.to_string_lossy();
    link_destinations(contents)
        .iter()
        .filter_map(|dest| resolve_destination(&file_str, dest, bundle))
        .any(|target| issue_target_is_terminal(store, &PathBuf::from(target), all_md))
}

fn issue_target_is_terminal(store: &dyn DocStore, target: &Path, all_md: &[PathBuf]) -> bool {
    if !all_md.iter().any(|p| p == target) {
        return false;
    }
    let Ok(contents) = store.read(target) else {
        return false;
    };
    if frontmatter_scalar(&contents, "type").as_deref() != Some("Issue") {
        return false;
    }
    let Some(status) = frontmatter_scalar(&contents, "status") else {
        return false;
    };
    doc_type::spec_for_frontmatter("Issue").is_some_and(|spec| spec.is_retired(&status))
}

#[cfg(test)]
mod tests;
