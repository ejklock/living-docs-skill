//! Four advisories on frontmatter states the CLI never writes but a hand edit
//! or an older release can leave behind: a status outside the type's
//! vocabulary, a record whose `type` is not its directory's type, a missing
//! `timestamp`, and a successor link on a record that is not superseded.
//! None of them fails the gate.

use super::records::{for_each_readable_record, frontmatter_scalar, has_frontmatter};
use super::{kinds, Reporter};
use crate::commands::next::numeric_prefix;
use crate::doc_type::{self, DocTypeSpec, Identity};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn check_frontmatter_advice(
    store: &dyn DocStore,
    bundle: &Path,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        if !has_frontmatter(&contents) {
            return;
        }
        for (kind, message) in findings(directory_spec(bundle, f), &contents) {
            reporter.advise(f, kind, message);
        }
    });
}

/// The registry type whose directory directly holds `file`, or `None` for a
/// file anywhere else in the bundle.
fn directory_spec(bundle: &Path, file: &Path) -> Option<&'static DocTypeSpec> {
    let inside = file.strip_prefix(bundle).unwrap_or(file);
    let mut parts = inside.components();
    let dir = parts.next()?.as_os_str().to_str()?;
    parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let spec = doc_type::spec_for_dir(dir)?;
    let name = file.file_name()?.to_str()?;
    let listed = match spec.identity {
        Identity::Numbered { .. } => numeric_prefix(name).is_some(),
        Identity::Named { .. } | Identity::Singleton { .. } => true,
    };
    listed.then_some(spec)
}

/// Every finding for one record, given the registry type of the directory
/// that holds it (if any) and its text.
fn findings(directory: Option<&DocTypeSpec>, contents: &str) -> Vec<(&'static str, String)> {
    let declared = frontmatter_scalar(contents, "type").unwrap_or_default();
    let spec = doc_type::spec_for_frontmatter(&declared);
    let status = frontmatter_scalar(contents, "status");
    let mut found = Vec::new();
    found.extend(status_finding(spec, status.as_deref()));
    found.extend(type_finding(directory, &declared));
    found.extend(timestamp_finding(spec, contents));
    found.extend(successor_finding(contents, status.as_deref()));
    found
}

fn status_finding(
    spec: Option<&DocTypeSpec>,
    status: Option<&str>,
) -> Option<(&'static str, String)> {
    let (spec, status) = (spec?, status?);
    if spec.status_vocabulary.is_empty()
        || status.eq_ignore_ascii_case("superseded")
        || spec.status_vocabulary.contains(&status)
    {
        return None;
    }
    let vocabulary = spec.status_vocabulary.join(", ");
    let canonical = spec
        .status_vocabulary
        .iter()
        .find(|entry| entry.eq_ignore_ascii_case(status));
    let fix = canonical.map_or(String::new(), |entry| format!("; write `{entry}`"));
    let message = format!(
        "{} status `{status}` is not in the vocabulary: {vocabulary}{fix}",
        spec.frontmatter
    );
    Some((kinds::STATUS_VOCABULARY, message))
}

fn type_finding(directory: Option<&DocTypeSpec>, declared: &str) -> Option<(&'static str, String)> {
    let directory = directory?;
    if declared.is_empty() || declared == directory.frontmatter {
        return None;
    }
    let message = format!(
        "declares `type: {declared}` in a directory of {} records",
        directory.frontmatter
    );
    Some((kinds::RECORD_TYPE, message))
}

fn timestamp_finding(spec: Option<&DocTypeSpec>, contents: &str) -> Option<(&'static str, String)> {
    let spec = spec?;
    if frontmatter_scalar(contents, "timestamp").is_some() {
        return None;
    }
    let message = format!(
        "{} record has no timestamp; run `living-docs manage fmt` to seed it",
        spec.frontmatter
    );
    Some((kinds::TIMESTAMP, message))
}

fn successor_finding(contents: &str, status: Option<&str>) -> Option<(&'static str, String)> {
    let successor = frontmatter_scalar(contents, "superseded_by")?;
    if status.is_some_and(|s| s.eq_ignore_ascii_case("superseded")) {
        return None;
    }
    let message = format!(
        "superseded_by: {successor} is set but the status is not Superseded; the record still reads as in force"
    );
    Some((kinds::STALE_SUCCESSOR, message))
}

#[cfg(test)]
mod tests;
