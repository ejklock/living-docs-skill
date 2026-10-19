//! The `MOVED-SOURCE` review queue: a warn-level finding when a record links
//! another record whose status has left the open/accepted state. Detection
//! reuses `check::links`' fence-aware extraction and resolution so the
//! markdown parsing lives in exactly one place; status/`superseded_by` reads
//! reuse `check::records::frontmatter_scalar`.
//!
//! The finding clears when the dependent's body links the successor
//! anywhere, when the resolved successor IS the dependent record itself, or
//! when the dependent's own status is terminal — Superseded for every type,
//! plus whatever else its doc type's registry row names as terminal — no
//! new annotation syntax, the acknowledgment IS the updated link.

use super::links::{link_destinations, resolve_destination};
use super::records::{frontmatter_scalar, is_reserved, record_id_matches};
use super::{file_name_str, kinds, Reporter};
use crate::doc_type;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_moved_source(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    let bundle_str = bundle.to_string_lossy();
    for f in all_md {
        check_file_moved_source(store, f, &bundle_str, reporter);
    }
}

fn check_file_moved_source(store: &dyn DocStore, f: &Path, bundle: &str, reporter: &mut Reporter) {
    if is_reserved(&file_name_str(f)) {
        return;
    }
    let Ok(contents) = store.read(f) else {
        return;
    };
    if is_closed_dependent(&contents) {
        return;
    }
    let file_str = f.to_string_lossy();
    for dest in link_destinations(&contents) {
        let Some(target) = resolve_destination(&file_str, &dest, bundle) else {
            continue;
        };
        check_target(
            store,
            f,
            &contents,
            bundle,
            &PathBuf::from(target),
            reporter,
        );
    }
}

fn check_target(
    store: &dyn DocStore,
    dependent: &Path,
    dependent_contents: &str,
    bundle: &str,
    target: &Path,
    reporter: &mut Reporter,
) {
    if is_reserved(&file_name_str(target)) {
        return;
    }
    let Ok(target_contents) = store.read(target) else {
        return;
    };
    let Some(status) = frontmatter_scalar(&target_contents, "status") else {
        return;
    };
    if !is_moved_status(&status) {
        return;
    }
    let successor = superseded_by(&status, &target_contents);
    if let Some(id) = &successor {
        if dependent_is_successor(dependent, target, id)
            || dependent_links_successor(dependent, dependent_contents, bundle, target, id)
        {
            return;
        }
    }
    reporter.advise(
        dependent,
        kinds::MOVED_SOURCE,
        moved_source_finding(dependent, target, &status, successor.as_deref()),
    );
}

/// True when the dependent's own status retires it: `Superseded` regardless
/// of `type` — a missing or unregistered `type` still counts, per the
/// issue's Decision — or a status `is_retired` accepts for a registered
/// `type`.
fn is_closed_dependent(contents: &str) -> bool {
    let Some(status) = frontmatter_scalar(contents, "status") else {
        return false;
    };
    if status.eq_ignore_ascii_case("superseded") {
        return true;
    }
    let Some(type_value) = frontmatter_scalar(contents, "type") else {
        return false;
    };
    doc_type::spec_for_frontmatter(&type_value).is_some_and(|spec| spec.is_retired(&status))
}

/// True when `successor_id` — the moved source's `superseded_by` value —
/// resolves, inside the source's own directory, to the dependent record
/// itself: a record that links its own predecessor has already
/// acknowledged the move by existing, so no further link update can clear
/// the finding.
fn dependent_is_successor(dependent: &Path, source: &Path, successor_id: &str) -> bool {
    let Some(dir) = source.parent() else {
        return false;
    };
    record_id_matches(dir, dependent, successor_id)
}

fn is_moved_status(status: &str) -> bool {
    matches!(
        status.to_lowercase().as_str(),
        "superseded" | "deprecated" | "rejected"
    )
}

fn superseded_by(status: &str, target_contents: &str) -> Option<String> {
    if !status.eq_ignore_ascii_case("superseded") {
        return None;
    }
    frontmatter_scalar(target_contents, "superseded_by")
}

fn dependent_links_successor(
    dependent: &Path,
    dependent_contents: &str,
    bundle: &str,
    source: &Path,
    successor_id: &str,
) -> bool {
    let Some(dir) = source.parent() else {
        return false;
    };
    let dependent_str = dependent.to_string_lossy();
    link_destinations(dependent_contents).iter().any(|dest| {
        resolve_destination(&dependent_str, dest, bundle)
            .map(|resolved| record_id_matches(dir, &PathBuf::from(resolved), successor_id))
            .unwrap_or(false)
    })
}

fn moved_source_finding(
    dependent: &Path,
    source: &Path,
    status: &str,
    successor: Option<&str>,
) -> String {
    let dependent_name = file_name_str(dependent);
    let source_name = file_name_str(source);
    match successor {
        Some(id) => format!(
            "MOVED-SOURCE: {dependent_name} links {source_name} whose status is {status}, superseded by {id}"
        ),
        None => format!("MOVED-SOURCE: {dependent_name} links {source_name} whose status is {status}"),
    }
}

#[cfg(test)]
mod tests;
