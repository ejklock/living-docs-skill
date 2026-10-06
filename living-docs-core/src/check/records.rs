//! Per-file record checks: OKF frontmatter/type, index-format, and the
//! supersede-chain (invariant 4). Mirrors the per-file loops in
//! `lint-docs.sh`. Every record's content is read through `DocStore::read`,
//! and the supersede-chain sibling lookup is driven by `all_md`
//! (`DocStore::list`'s own enumeration) rather than a filesystem re-scan, so
//! both invariants validate whichever backend `check::compile` is given.

use super::{file_name_str, Reporter};
use crate::doc_type;
use crate::frontmatter::{frontmatter_block, read_scalar_strict};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

pub(crate) fn is_reserved(basename: &str) -> bool {
    basename == "index.md" || basename == "log.md"
}

pub(crate) fn has_frontmatter(contents: &str) -> bool {
    contents.lines().next() == Some("---")
}

/// Reads a top-level scalar from `contents`' leading `---`-fenced YAML
/// frontmatter block via the crate's shared strict reader — no
/// `raw_scalar_line` fallback, so an invalid-YAML plain scalar the lenient
/// `crate::frontmatter::read_scalar_from_str` would recover instead reports
/// as absent here.
pub(crate) fn frontmatter_scalar(contents: &str, key: &str) -> Option<String> {
    read_scalar_strict(frontmatter_block(contents)?, key)
}

/// Every non-reserved `.md` needs frontmatter with a non-empty top-level `type`.
/// `index.md`/`log.md` carry no frontmatter, except the bundle-root `index.md`,
/// which may declare `okf_version`.
pub(crate) fn check_frontmatter_and_format(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    root_index: &Path,
    reporter: &mut Reporter,
) {
    for f in all_md {
        let base = file_name_str(f);
        let contents = store.read(f).unwrap_or_default();
        if is_reserved(&base) {
            check_reserved_file(f, &base, root_index, &contents, reporter);
        } else {
            check_concept_file(f, &contents, reporter);
        }
    }
}

fn check_reserved_file(
    f: &Path,
    base: &str,
    root_index: &Path,
    contents: &str,
    reporter: &mut Reporter,
) {
    if !has_frontmatter(contents) {
        return;
    }
    if f == root_index {
        if frontmatter_scalar(contents, "okf_version").is_none() {
            reporter.report(f, "bundle-root index.md frontmatter lacks okf_version");
        }
    } else {
        reporter.report(f, format!("{base} must not carry frontmatter (OKF §6)"));
    }
}

fn check_concept_file(f: &Path, contents: &str, reporter: &mut Reporter) {
    if !has_frontmatter(contents) {
        reporter.report(f, "missing OKF frontmatter (needs a non-empty 'type')");
        return;
    }
    if frontmatter_scalar(contents, "type").is_none() {
        reporter.report(f, "frontmatter has no non-empty 'type'");
    }
}

/// A `status: Superseded` record (case-insensitive) needs a non-empty
/// `superseded_by` resolving to a sibling `<NNNN>-*.md` or `<NNNN>.md`
/// record. The sibling lookup matches against `all_md` — the same
/// enumeration `check::compile` got from `DocStore::list` — instead of
/// re-scanning the directory on disk, so a target the active backend never
/// enumerates is caught even when a same-named file still exists on disk.
pub(crate) fn check_supersede_chain(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        if !has_frontmatter(&contents) {
            return;
        }
        let Some(status) = frontmatter_scalar(&contents, "status") else {
            return;
        };
        if status.to_lowercase() == "superseded" {
            check_supersede_target(f, &contents, all_md, reporter);
        }
    });
}

/// Visits every non-reserved, store-readable record in `all_md` — the
/// skeleton `check_supersede_chain`, `check_heading_matches_title` and
/// `check_owner_requirement` all walk before applying their own rule.
pub(super) fn for_each_readable_record<'a>(
    store: &dyn DocStore,
    all_md: &'a [PathBuf],
    mut visit: impl FnMut(&'a Path, String),
) {
    for f in all_md {
        if is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        visit(f, contents);
    }
}

fn check_supersede_target(f: &Path, contents: &str, all_md: &[PathBuf], reporter: &mut Reporter) {
    let Some(sb) = frontmatter_scalar(contents, "superseded_by") else {
        reporter.report(
            f,
            "status: Superseded but superseded_by is empty (invariant 4)",
        );
        return;
    };
    let dir = f.parent().unwrap_or_else(|| Path::new("."));
    if !sibling_record_exists(dir, &sb, all_md) {
        reporter.report(
            f,
            format!(
                "superseded_by: {sb} has no matching record in {} (invariant 4)",
                dir.display()
            ),
        );
    }
}

/// A record whose heading disagrees with its frontmatter `title` is an
/// advisory (ADR 0061): `set title` writes both from one value, so a
/// divergence means one of them was hand-edited. It stays advisory because a
/// heading shortened by hand is a legitimate historical shape — the finding
/// names the drift without failing a corpus that predates the rule.
pub(crate) fn check_heading_matches_title(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        if is_retired_record(&contents) {
            return;
        }
        let (Some(title), Some(heading)) = (
            frontmatter_scalar(&contents, "title"),
            heading_title(&contents),
        ) else {
            return;
        };
        if heading != title {
            reporter.advise(
                f,
                "heading",
                format!("HEADING '{heading}' disagrees with title '{title}'"),
            );
        }
    });
}

/// The record's heading text, with the heading marks and the `NNNN.` number
/// prefix the templates write stripped, so it compares against the bare
/// frontmatter `title`.
fn heading_title(contents: &str) -> Option<String> {
    let body = contents.split("\n---\n").nth(1)?;
    let heading = body.lines().find(|line| line.starts_with('#'))?;
    Some(crate::sections::heading_text(heading).to_owned())
}

/// Whether `contents`' own `type`/`status` frontmatter marks the record
/// retired, through the shared [`doc_type::DocTypeSpec::is_retired`]
/// predicate (ADR 0063). Absent either key, or an unregistered `type`, the
/// record is judged live — there is nothing to retire it against.
pub(super) fn is_retired_record(contents: &str) -> bool {
    let Some(doc_type) = frontmatter_scalar(contents, "type") else {
        return false;
    };
    let Some(status) = frontmatter_scalar(contents, "status") else {
        return false;
    };
    doc_type::spec_for_frontmatter(&doc_type).is_some_and(|spec| spec.is_retired(&status))
}

/// A record whose doctype registry row sets `requires_owner` and whose
/// frontmatter carries no `owner:` value is a finding: an advisory by
/// default, or an invariant violation under `require_owner`. The
/// requirement is read from each record's own resolved
/// [`doc_type::DocTypeSpec`], never a hardcoded type list, so a doctype
/// change here needs no edit at this call site.
pub(crate) fn check_owner_requirement(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    require_owner: bool,
    reporter: &mut Reporter,
) {
    for_each_readable_record(store, all_md, |f, contents| {
        if !has_frontmatter(&contents) {
            return;
        }
        report_missing_owner(f, &contents, require_owner, reporter);
    });
}

fn report_missing_owner(f: &Path, contents: &str, require_owner: bool, reporter: &mut Reporter) {
    let Some(doc_type) = frontmatter_scalar(contents, "type") else {
        return;
    };
    let Some(spec) = doc_type::spec_for_frontmatter(&doc_type) else {
        return;
    };
    if !spec.requires_owner || frontmatter_scalar(contents, "owner").is_some() {
        return;
    }

    let message = format!("{} record has no owner", spec.frontmatter);
    if require_owner {
        reporter.report(f, message);
    } else {
        reporter.advise(f, "owner", message);
    }
}

fn sibling_record_exists(dir: &Path, sb: &str, all_md: &[PathBuf]) -> bool {
    all_md.iter().any(|p| record_id_matches(dir, p, sb))
}

/// True when `path` is `dir`'s bare `<id>.md` or a `<id>-*.md` sibling —
/// the two forms a record id may resolve to inside one directory. Shared by
/// the supersede-chain sibling lookup and the moved-source successor-link
/// clearing rule, so both read the same identity rule.
pub(crate) fn record_id_matches(dir: &Path, path: &Path, id: &str) -> bool {
    if path.parent() != Some(dir) {
        return false;
    }
    let bare = dir.join(format!("{id}.md"));
    let prefix = format!("{id}-");
    path == bare || file_name_str(path).starts_with(&prefix)
}

#[cfg(test)]
mod tests;
