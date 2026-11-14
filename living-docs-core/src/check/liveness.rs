//! Mechanical record liveness: `check` emits a `stale-proposed` advisory for
//! an ADR still at its seed status (`Proposed`) that a terminal or superseded
//! issue declares it implements — the word `Implements` (any case) followed
//! by a markdown link that resolves to the ADR, in the issue body. A link from
//! the ADR to an issue is history and never counts. The exit code never moves,
//! same posture as `SIZE` and `MOVED-SOURCE`.

use super::links::{link_starts, resolve_destination, LinkKind};
use super::records::{frontmatter_scalar, is_reserved};
use super::{file_name_str, kinds, Reporter};
use crate::doc_type;
use crate::record::strip_frontmatter;
use crate::store::DocStore;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) fn check_liveness(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let bundle_str = bundle.to_string_lossy();
    let declared = declared_implemented(store, &bundle_str, all_md);
    for f in all_md {
        if is_stale_proposed(store, f, &declared) {
            reporter.advise(
                f,
                kinds::LIVENESS,
                "LIVENESS stale-proposed: seed status but the linked issue is terminal — settle or supersede",
            );
        }
    }
}

/// True when `f` is an ADR still at its seed status (`Proposed`) that a
/// terminal issue declares it implements.
fn is_stale_proposed(store: &dyn DocStore, f: &Path, declared: &HashSet<String>) -> bool {
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
    is_seed_status(&doc_type, &status) && declared.contains(f.to_string_lossy().as_ref())
}

/// Only ADR carries a liveness signal: a decision record's currency is
/// checkable against the issue that implements it.
fn is_liveness_type(doc_type: &str) -> bool {
    doc_type == "ADR"
}

/// The seed status is `status_vocabulary[0]` — `Proposed` for ADR — the state
/// a record is born in and should leave once its work lands (registry-sourced).
fn is_seed_status(doc_type: &str, status: &str) -> bool {
    doc_type::spec_for_frontmatter(doc_type)
        .and_then(|spec| spec.status_vocabulary.first())
        .is_some_and(|seed| seed.eq_ignore_ascii_case(status))
}

/// The paths every terminal issue declares it implements, collected once per
/// run so each ADR is a set lookup rather than a re-read of every issue.
fn declared_implemented(store: &dyn DocStore, bundle: &str, all_md: &[PathBuf]) -> HashSet<String> {
    all_md
        .iter()
        .filter_map(|f| {
            let contents = store.read(f).ok()?;
            is_terminal_issue(&contents).then(|| implements_targets(f, &contents, bundle))
        })
        .flatten()
        .collect()
}

fn is_terminal_issue(contents: &str) -> bool {
    if frontmatter_scalar(contents, "type").as_deref() != Some("Issue") {
        return false;
    }
    let Some(status) = frontmatter_scalar(contents, "status") else {
        return false;
    };
    doc_type::spec_for_frontmatter("Issue").is_some_and(|spec| spec.is_retired(&status))
}

/// Resolved targets of every link that directly follows the word `Implements`
/// (any case, optional spaces) in the body, frontmatter excluded. Fenced code
/// never yields a link event.
fn implements_targets(file: &Path, contents: &str, bundle: &str) -> Vec<String> {
    let file_str = file.to_string_lossy();
    let body = strip_frontmatter(contents);
    link_starts(body)
        .into_iter()
        .filter(|(start, kind, _)| *kind == LinkKind::Link && follows_implements(&body[..*start]))
        .filter_map(|(_, _, dest)| resolve_destination(&file_str, &dest, bundle))
        .collect()
}

fn follows_implements(before: &str) -> bool {
    const WORD: &str = "implements";
    let head = before.trim_end_matches(' ');
    let Some(start) = head.len().checked_sub(WORD.len()) else {
        return false;
    };
    head.get(start..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(WORD))
        && !head[..start].ends_with(|c: char| c.is_alphanumeric())
}

#[cfg(test)]
mod tests;
