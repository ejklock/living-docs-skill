//! Per-file record checks: OKF frontmatter/type, index-format, and the
//! supersede-chain (invariant 4). Mirrors the per-file loops in `lint-docs.sh`.

use super::{file_name_str, Reporter};
use crate::frontmatter;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn is_reserved(basename: &str) -> bool {
    basename == "index.md" || basename == "log.md"
}

pub(crate) fn has_frontmatter(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(contents) => contents.lines().next() == Some("---"),
        Err(_) => false,
    }
}

/// Every non-reserved `.md` needs frontmatter with a non-empty top-level `type`.
/// `index.md`/`log.md` carry no frontmatter, except the bundle-root `index.md`,
/// which may declare `okf_version`.
pub(crate) fn check_frontmatter_and_format(
    all_md: &[PathBuf],
    root_index: &Path,
    reporter: &mut Reporter,
) {
    for f in all_md {
        let base = file_name_str(f);
        if is_reserved(&base) {
            check_reserved_file(f, &base, root_index, reporter);
        } else {
            check_concept_file(f, reporter);
        }
    }
}

fn check_reserved_file(f: &Path, base: &str, root_index: &Path, reporter: &mut Reporter) {
    if !has_frontmatter(f) {
        return;
    }
    if f == root_index {
        if frontmatter::read_scalar(f, "okf_version").is_none() {
            reporter.report(f, "bundle-root index.md frontmatter lacks okf_version");
        }
    } else {
        reporter.report(f, format!("{base} must not carry frontmatter (OKF §6)"));
    }
}

fn check_concept_file(f: &Path, reporter: &mut Reporter) {
    if !has_frontmatter(f) {
        reporter.report(f, "missing OKF frontmatter (needs a non-empty 'type')");
        return;
    }
    if frontmatter::read_scalar(f, "type").is_none() {
        reporter.report(f, "frontmatter has no non-empty 'type'");
    }
}

/// A `status: Superseded` record (case-insensitive) needs a non-empty
/// `superseded_by` resolving to a sibling `<NNNN>-*.md` or `<NNNN>.md` file.
pub(crate) fn check_supersede_chain(all_md: &[PathBuf], reporter: &mut Reporter) {
    for f in all_md {
        if is_reserved(&file_name_str(f)) || !has_frontmatter(f) {
            continue;
        }
        let Some(status) = frontmatter::read_scalar(f, "status") else {
            continue;
        };
        if status.to_lowercase() == "superseded" {
            check_supersede_target(f, reporter);
        }
    }
}

fn check_supersede_target(f: &Path, reporter: &mut Reporter) {
    let Some(sb) = frontmatter::read_scalar(f, "superseded_by") else {
        reporter.report(
            f,
            "status: Superseded but superseded_by is empty (invariant 4)",
        );
        return;
    };
    let dir = f.parent().unwrap_or_else(|| Path::new("."));
    if !sibling_record_exists(dir, &sb) {
        reporter.report(
            f,
            format!(
                "superseded_by: {sb} has no matching record in {} (invariant 4)",
                dir.display()
            ),
        );
    }
}

fn sibling_record_exists(dir: &Path, sb: &str) -> bool {
    if dir.join(format!("{sb}.md")).is_file() {
        return true;
    }
    let prefix = format!("{sb}-");
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .any(|entry| {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            name.starts_with(&prefix) && name.ends_with(".md")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_reserved_matches_index_and_log_only() {
        assert!(is_reserved("index.md"));
        assert!(is_reserved("log.md"));
        assert!(!is_reserved("foo.md"));
    }

    #[test]
    fn sibling_record_exists_matches_dash_prefixed_and_bare_forms() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("living-docs-records-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("0007-old.md"), "# Old\n").unwrap();

        assert!(sibling_record_exists(&dir, "0007"));
        assert!(!sibling_record_exists(&dir, "9999"));

        fs::write(dir.join("0042.md"), "# Bare\n").unwrap();
        assert!(sibling_record_exists(&dir, "0042"));

        let _ = fs::remove_dir_all(&dir);
    }
}
