//! `living-docs leak-gate <bundle>`: fails closed on ADR 0010's first two
//! leak classes over an already-exported bundle directory — a private (or
//! visibility-absent) doc that leaked into the bundle, and a link from a
//! published doc to a target withheld from the bundle. The secret/PII regex
//! class is a follow-up slice.
//!
//! The dangling-link scan reuses `check::links`'s destination extraction and
//! resolution rather than re-implementing it, so the two invariants ("does
//! this link exist" for `check`, "was this link's target withheld" here)
//! never drift apart. This command keeps its own small violation collector
//! instead of `check::Reporter` — the two commands report unrelated
//! invariants and have no reason to share output shape.

use crate::check::file_name_str;
use crate::check::links::{link_destinations, resolve_destination};
use crate::frontmatter;
use crate::store::DocStore;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    let all_md = store.list(bundle).unwrap_or_default();
    let bundle_str = bundle.to_string_lossy();
    let mut violations = Vec::new();

    for path in &all_md {
        collect_private_present_violation(store, path, &mut violations);
        collect_dangling_link_violations(store, path, &bundle_str, &mut violations);
    }

    report(violations)
}

const DEFAULT_VISIBILITY: &str = "private";
const BUNDLE_VISIBLE: [&str; 2] = ["public", "showcase"];

fn effective_visibility(contents: &str) -> String {
    frontmatter::read_scalar_from_str(contents, "visibility")
        .unwrap_or_else(|| DEFAULT_VISIBILITY.to_string())
}

fn is_bundle_visible(visibility: &str) -> bool {
    BUNDLE_VISIBLE.contains(&visibility)
}

/// `index.md` and `log.md` carry no frontmatter by design (OKF, ADR 0007) —
/// they are directory listings, not records — so they are exempt from the
/// private-doc-present check rather than defaulting to "leaked private".
fn is_reserved_listing_file(path: &Path) -> bool {
    let name = file_name_str(path);
    name == "index.md" || name == "log.md"
}

fn collect_private_present_violation(
    store: &dyn DocStore,
    path: &Path,
    violations: &mut Vec<(PathBuf, String)>,
) {
    if is_reserved_listing_file(path) {
        return;
    }
    let Ok(contents) = store.read(path) else {
        return;
    };
    let visibility = effective_visibility(&contents);
    if !is_bundle_visible(&visibility) {
        violations.push((
            path.to_path_buf(),
            format!("private doc present in bundle (visibility: {visibility})"),
        ));
    }
}

fn collect_dangling_link_violations(
    store: &dyn DocStore,
    path: &Path,
    bundle: &str,
    violations: &mut Vec<(PathBuf, String)>,
) {
    let Ok(contents) = store.read(path) else {
        return;
    };
    let file_str = path.to_string_lossy();
    for dest in link_destinations(&contents) {
        let Some(target) = resolve_destination(&file_str, &dest, bundle) else {
            continue;
        };
        if !Path::new(&target).exists() {
            violations.push((
                path.to_path_buf(),
                format!("link to withheld doc -> {target}"),
            ));
        }
    }
}

fn report(violations: Vec<(PathBuf, String)>) -> ExitCode {
    for (file, message) in &violations {
        println!("  {:<44} {message}", file.display().to_string());
    }
    println!();
    if violations.is_empty() {
        println!("OK — no leaks detected.");
        ExitCode::SUCCESS
    } else {
        println!("FAIL — {} leak(s) detected.", violations.len());
        ExitCode::from(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::fs;
    use std::io;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn exit_code_is_success(code: ExitCode) -> bool {
        format!("{code:?}") == format!("{:?}", ExitCode::SUCCESS)
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

    struct TempBundle {
        root: PathBuf,
    }

    impl TempBundle {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let root =
                std::env::temp_dir().join(format!("living-docs-core-leak-gate-{label}-{nanos}"));
            fs::create_dir_all(root.join("adr")).expect("create temp bundle adr dir");
            Self { root }
        }
    }

    impl Drop for TempBundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn leak_gate_fails_when_a_private_doc_is_present_in_the_bundle() {
        let bundle = TempBundle::new("private-present");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-secret.md"),
            "---\ntype: ADR\nvisibility: private\n---\n# Secret\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_names_the_offending_file_and_its_visibility() {
        let bundle = TempBundle::new("private-message");
        let doc_path = bundle.root.join("adr").join("0001-secret.md");
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            "---\ntype: ADR\n---\n# Absent Visibility\n".to_string(),
        );
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_private_present_violation(&store, &doc_path, &mut violations);

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, doc_path);
        assert!(violations[0].1.contains("private"));
    }

    #[test]
    fn leak_gate_exempts_reserved_index_and_log_files_from_the_private_doc_check() {
        let bundle = TempBundle::new("reserved-exempt");
        let mut files = BTreeMap::new();
        files.insert(bundle.root.join("index.md"), "# Index\n".to_string());
        files.insert(bundle.root.join("log.md"), "# Log\n".to_string());
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_fails_when_a_link_targets_a_doc_withheld_from_the_bundle() {
        let bundle = TempBundle::new("dangling-link");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n\n[missing](./0002-missing.md)\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(!exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_dangling_link_message_locates_the_link_and_its_target() {
        let bundle = TempBundle::new("dangling-link-message");
        let doc_path = bundle.root.join("adr").join("0001-doc.md");
        let mut files = BTreeMap::new();
        files.insert(
            doc_path.clone(),
            "[missing](./0002-missing.md)\n".to_string(),
        );
        let store = MapStore { files };
        let mut violations = Vec::new();

        collect_dangling_link_violations(
            &store,
            &doc_path,
            &bundle.root.to_string_lossy(),
            &mut violations,
        );

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].0, doc_path);
        assert!(violations[0].1.contains("0002-missing.md"));
    }

    #[test]
    fn leak_gate_passes_a_clean_bundle_where_every_doc_is_public_and_every_link_resolves() {
        let bundle = TempBundle::new("clean-bundle");
        let target_path = bundle.root.join("adr").join("0002-target.md");
        fs::write(
            &target_path,
            "---\ntype: ADR\nvisibility: public\n---\n# Target\n",
        )
        .expect("write real target file backing the resolved link");

        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nvisibility: public\n---\n# Doc\n\n[target](./0002-target.md)\n"
                .to_string(),
        );
        files.insert(
            target_path.clone(),
            "---\ntype: ADR\nvisibility: public\n---\n# Target\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }

    #[test]
    fn leak_gate_passes_an_empty_bundle() {
        let bundle = TempBundle::new("empty-bundle");
        let store = MapStore {
            files: BTreeMap::new(),
        };

        let code = run(&store, &bundle.root);

        assert!(exit_code_is_success(code));
    }
}
