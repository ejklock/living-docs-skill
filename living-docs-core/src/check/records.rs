//! Per-file record checks: OKF frontmatter/type, index-format, and the
//! supersede-chain (invariant 4). Mirrors the per-file loops in
//! `lint-docs.sh`. Every record's content is read through `DocStore::read`,
//! and the supersede-chain sibling lookup is driven by `all_md`
//! (`DocStore::list`'s own enumeration) rather than a filesystem re-scan, so
//! both invariants validate whichever backend `check::run` is given.

use super::{file_name_str, Reporter};
use crate::store::DocStore;
use serde_yaml::Value;
use std::path::{Path, PathBuf};

pub(crate) fn is_reserved(basename: &str) -> bool {
    basename == "index.md" || basename == "log.md"
}

pub(crate) fn has_frontmatter(contents: &str) -> bool {
    contents.lines().next() == Some("---")
}

/// Reads a top-level scalar from `contents`' leading `---`-fenced YAML
/// frontmatter block. Mirrors `crate::frontmatter::read_scalar`, operating
/// on already-read content instead of a path so callers can source it from
/// any `DocStore`.
fn frontmatter_scalar(contents: &str, key: &str) -> Option<String> {
    let block = extract_frontmatter_block(contents)?;
    let document: Value = serde_yaml::from_str(block).ok()?;
    let mapping = document.as_mapping()?;
    let value = mapping.get(Value::String(key.to_string()))?;
    scalar_to_string(value)
}

fn extract_frontmatter_block(contents: &str) -> Option<&str> {
    let rest = contents.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn scalar_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
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
    if let Some(visibility) = frontmatter_scalar(contents, "visibility") {
        if !is_valid_visibility(&visibility) {
            reporter.report(
                f,
                format!(
                    "invalid visibility '{visibility}' (allowed: private|public|showcase; absent means private)"
                ),
            );
        }
    }
}

/// Domain check for a *present* `visibility` value (ADR 0009). Absence is
/// handled upstream by the `Option` branch in `check_concept_file` and never
/// reaches this predicate — default-deny means an absent field is always
/// valid, so only a present value needs validating against the domain.
fn is_valid_visibility(value: &str) -> bool {
    matches!(value, "private" | "public" | "showcase")
}

/// A `status: Superseded` record (case-insensitive) needs a non-empty
/// `superseded_by` resolving to a sibling `<NNNN>-*.md` or `<NNNN>.md`
/// record. The sibling lookup matches against `all_md` — the same
/// enumeration `check::run` got from `DocStore::list` — instead of
/// re-scanning the directory on disk, so a target the active backend never
/// enumerates is caught even when a same-named file still exists on disk.
pub(crate) fn check_supersede_chain(
    store: &dyn DocStore,
    all_md: &[PathBuf],
    reporter: &mut Reporter,
) {
    for f in all_md {
        if is_reserved(&file_name_str(f)) {
            continue;
        }
        let Ok(contents) = store.read(f) else {
            continue;
        };
        if !has_frontmatter(&contents) {
            continue;
        }
        let Some(status) = frontmatter_scalar(&contents, "status") else {
            continue;
        };
        if status.to_lowercase() == "superseded" {
            check_supersede_target(f, &contents, all_md, reporter);
        }
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

fn sibling_record_exists(dir: &Path, sb: &str, all_md: &[PathBuf]) -> bool {
    let bare = dir.join(format!("{sb}.md"));
    let prefix = format!("{sb}-");
    all_md
        .iter()
        .any(|p| p.parent() == Some(dir) && (p == &bare || file_name_str(p).starts_with(&prefix)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::io;
    use std::process::ExitCode;

    #[test]
    fn is_reserved_matches_index_and_log_only() {
        assert!(is_reserved("index.md"));
        assert!(is_reserved("log.md"));
        assert!(!is_reserved("foo.md"));
    }

    #[test]
    fn sibling_record_exists_matches_dash_prefixed_and_bare_forms() {
        let dir = Path::new("docs/adr");
        let all_md = vec![dir.join("0007-old.md"), dir.join("0042.md")];

        assert!(sibling_record_exists(dir, "0007", &all_md));
        assert!(!sibling_record_exists(dir, "9999", &all_md));
        assert!(sibling_record_exists(dir, "0042", &all_md));
    }

    #[test]
    fn sibling_record_exists_ignores_matches_outside_the_directory() {
        let dir = Path::new("docs/adr");
        let all_md = vec![Path::new("docs/bdr").join("0007-old.md")];

        assert!(!sibling_record_exists(dir, "0007", &all_md));
    }

    struct MapStore {
        files: BTreeMap<PathBuf, String>,
    }

    impl DocStore for MapStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .files
                .keys()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }
    }

    fn exit_code_is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
    }

    #[test]
    fn check_frontmatter_and_format_accepts_content_the_store_serves_with_no_disk_backing() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntype: ADR\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        assert!(exit_code_is_success(reporter.finish(1)));
    }

    #[test]
    fn check_frontmatter_and_format_reports_content_the_store_serves_as_missing_frontmatter() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "# No frontmatter\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        assert!(!exit_code_is_success(reporter.finish(1)));
    }

    #[test]
    fn check_supersede_chain_reports_a_target_absent_from_all_md() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-old.md"),
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-old.md")];
        let mut reporter = Reporter::new();

        check_supersede_chain(&store, &all_md, &mut reporter);

        assert!(!exit_code_is_success(reporter.finish(1)));
    }

    #[test]
    fn check_frontmatter_and_format_accepts_a_valid_visibility_value() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        assert!(exit_code_is_success(reporter.finish(1)));
    }

    #[test]
    fn check_frontmatter_and_format_reports_a_misspelled_visibility_value() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntype: ADR\nvisibility: pubic\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        let code = reporter.finish(1);
        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn check_frontmatter_and_format_reports_the_offending_value_and_allowed_domain() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntype: ADR\nvisibility: pubic\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        let messages: Vec<&str> = reporter
            .violations
            .iter()
            .map(|(_, message)| message.as_str())
            .collect();
        assert!(messages
            .iter()
            .any(|message| message.contains("invalid visibility 'pubic'")));
        assert!(messages
            .iter()
            .any(|message| message
                .contains("allowed: private|public|showcase; absent means private")));
    }

    #[test]
    fn check_frontmatter_and_format_treats_absent_visibility_as_silent_pass() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntype: ADR\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        assert!(exit_code_is_success(reporter.finish(1)));
    }

    #[test]
    fn check_frontmatter_and_format_treats_absent_visibility_as_silent_pass_on_an_untyped_doc() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-title.md"),
            "---\ntitle: No type here\n---\n# Title\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![PathBuf::from("/bundle/adr/0001-title.md")];
        let root_index = PathBuf::from("/bundle/index.md");
        let mut reporter = Reporter::new();

        check_frontmatter_and_format(&store, &all_md, &root_index, &mut reporter);

        let messages: Vec<&str> = reporter
            .violations
            .iter()
            .map(|(_, message)| message.as_str())
            .collect();
        assert!(messages
            .iter()
            .any(|message| message.contains("non-empty 'type'")));
        assert!(!messages
            .iter()
            .any(|message| message.contains("visibility")));
    }

    #[test]
    fn is_valid_visibility_accepts_exactly_the_domain_values() {
        assert!(is_valid_visibility("private"));
        assert!(is_valid_visibility("public"));
        assert!(is_valid_visibility("showcase"));
        assert!(!is_valid_visibility("Public"));
        assert!(!is_valid_visibility("pubic"));
        assert!(!is_valid_visibility(""));
    }

    #[test]
    fn check_supersede_chain_passes_when_the_target_is_present_in_all_md() {
        let mut files = BTreeMap::new();
        files.insert(
            PathBuf::from("/bundle/adr/0001-old.md"),
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Old\n".to_string(),
        );
        files.insert(
            PathBuf::from("/bundle/adr/0002-new.md"),
            "---\ntype: ADR\nstatus: Accepted\n---\n# New\n".to_string(),
        );
        let store = MapStore { files };
        let all_md = vec![
            PathBuf::from("/bundle/adr/0001-old.md"),
            PathBuf::from("/bundle/adr/0002-new.md"),
        ];
        let mut reporter = Reporter::new();

        check_supersede_chain(&store, &all_md, &mut reporter);

        assert!(exit_code_is_success(reporter.finish(2)));
    }
}
