//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format,
//! directory-index membership, bundle-root reachability, supersede-chain
//! integrity, local link/image validity via `pulldown-cmark`, and (ADR 0013)
//! ```mermaid``` fence validation in-process via `merman-core`.
//!
//! Every record's content (`records`, `links`) is read through
//! `DocStore::read`, so `check` validates whichever backend `run` is given.
//! `index.md`/`log.md` are excluded from the record domain by design (never
//! synced to `db-store`, see `db_store::record::is_reserved`), so
//! `check::graph`'s directory-index parsing reads them straight from disk —
//! that traversal is documented at its own call site.

mod canonical;
mod graph;
pub(crate) mod links;
mod mermaid;
mod records;
mod size;

use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences under
/// `paths`, skipping every other invariant. See `mermaid::run_mermaid_only`.
pub fn run_mermaid_only(paths: &[PathBuf]) -> ExitCode {
    mermaid::run_mermaid_only(paths)
}

pub fn run(store: &dyn DocStore, bundle: &Path) -> ExitCode {
    if !bundle.is_dir() {
        eprintln!(
            "living-docs check: bundle root not found: {}",
            bundle.display()
        );
        eprintln!(
            "       run from the repo root, or pass the docs directory: living-docs check path/to/docs"
        );
        return ExitCode::from(2);
    }

    println!("Living Docs lint — bundle: {}", bundle.display());
    println!();

    let mut reporter = Reporter::new();
    let doc_count = run_all_checks(store, bundle, &mut reporter);

    reporter.finish(doc_count)
}

/// Every invariant `check` validates, without the surrounding
/// `bundle.is_dir()` guard, header, or verdict rendering — shared by
/// [`run`] (which prints the verdict) and [`check_violations`] (which
/// returns the raw list, for a caller like `db_store::DbDocStore::write_checked`
/// that gates a write on the same invariants without printing anything).
/// Returns the number of docs `store` enumerated under `bundle`.
fn run_all_checks(store: &dyn DocStore, bundle: &Path, reporter: &mut Reporter) -> usize {
    let all_md = store.list(bundle).unwrap_or_default();
    let root_index = bundle.join("index.md");

    if !root_index.is_file() {
        reporter.report(&root_index, "missing bundle-root index.md (invariant 3)");
    }

    records::check_frontmatter_and_format(store, &all_md, &root_index, reporter);
    graph::check_directory_membership(bundle, &all_md, reporter);
    graph::check_reachability(bundle, &root_index, &all_md, reporter);
    links::check_links(store, bundle, &all_md, reporter);
    records::check_supersede_chain(store, &all_md, reporter);
    canonical::check_canonical_frontmatter(store, &all_md, reporter);

    mermaid::check_bundle(&all_md, reporter);
    size::check_body_size(store, &all_md, reporter);

    all_md.len()
}

/// The same invariants [`run`] validates, returned as a plain violation list
/// rather than printed and turned into an [`ExitCode`] — the mechanism a
/// transactional write+check verb (e.g. `db_store::DbDocStore::write_checked`)
/// needs to gate a commit on `check` passing without emitting `check`'s own
/// stdout report.
pub fn check_violations(store: &dyn DocStore, bundle: &Path) -> Vec<(String, String)> {
    let mut reporter = Reporter::new();
    run_all_checks(store, bundle, &mut reporter);
    reporter.into_violations()
}

pub(crate) fn file_name_str(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

pub(crate) fn collect_md_files(bundle: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_md_files(bundle, &mut out);
    out.sort();
    out
}

fn walk_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            walk_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

/// Collects violations and renders the final report + exit code, mirroring
/// `report()` and the verdict block of `lint-docs.sh`. Advisories (issue
/// 0009) print alongside violations but never touch the exit code.
pub(crate) struct Reporter {
    violations: Vec<(String, String)>,
    advisories: Vec<(String, String)>,
}

impl Reporter {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            advisories: Vec::new(),
        }
    }

    pub(crate) fn report(&mut self, file: &Path, message: impl Into<String>) {
        self.violations
            .push((file.display().to_string(), message.into()));
    }

    pub(crate) fn advise(&mut self, file: &Path, message: impl Into<String>) {
        self.advisories
            .push((file.display().to_string(), message.into()));
    }

    fn into_violations(self) -> Vec<(String, String)> {
        self.violations
    }

    fn finish(self, doc_count: usize) -> ExitCode {
        for (file, message) in &self.advisories {
            println!("  {file:<44} {message}");
        }
        if !self.advisories.is_empty() {
            println!();
        }
        for (file, message) in &self.violations {
            println!("  {file:<44} {message}");
        }
        println!();
        if self.violations.is_empty() {
            println!("OK — {doc_count} docs, no invariant violations.");
            ExitCode::SUCCESS
        } else {
            println!(
                "FAIL — {} violation(s) across {doc_count} docs.",
                self.violations.len()
            );
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MapStore;
    use std::collections::BTreeMap;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn file_name_str_returns_the_basename() {
        assert_eq!(file_name_str(Path::new("docs/adr/index.md")), "index.md");
    }

    #[test]
    fn reporter_with_no_violations_reports_clean_and_exits_zero() {
        let reporter = Reporter::new();
        let code = reporter.finish(3);
        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    struct ScratchBundle {
        root: PathBuf,
    }

    impl ScratchBundle {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!("living-docs-check-mod-{label}-{nanos}"));
            fs::create_dir_all(root.join("adr")).expect("create scratch adr dir");
            fs::write(
                root.join("index.md"),
                "# Index\n\n- [ADRs](/adr/index.md)\n",
            )
            .expect("write scratch root index");
            fs::write(
                root.join("adr").join("index.md"),
                "# ADR Index\n\n- [Doc](/adr/0001-doc.md)\n- [Other](/adr/0002-other.md)\n",
            )
            .expect("write scratch adr index");
            Self { root }
        }
    }

    impl Drop for ScratchBundle {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    /// A record served only by the store (never written to disk) must still
    /// pass `check` — proving content reads route through `DocStore::read`
    /// rather than falling back to `std::fs::read_to_string` at the same
    /// path.
    #[test]
    fn run_reads_record_content_through_the_store_even_when_no_file_backs_it_on_disk() {
        let bundle = ScratchBundle::new("store-only-record");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\ntitle: Doc\ndescription: \"\"\n---\n\n# Doc\n\nBody.\n".to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert_eq!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }

    /// A supersede target that a real file on disk would satisfy, but that
    /// the active store never enumerates, must still fail `check` — proving
    /// the sibling lookup is driven by the store's own enumeration
    /// (`DocStore::list`), not a filesystem re-scan.
    #[test]
    fn run_reports_a_supersede_target_the_store_omits_even_though_a_same_named_file_exists_on_disk()
    {
        let bundle = ScratchBundle::new("store-omits-target");
        fs::write(
            bundle.root.join("adr").join("0002-other.md"),
            "---\ntype: ADR\ntitle: Other\n---\n# Other\n\nBody.\n",
        )
        .expect("write on-disk supersede target");
        let mut files = BTreeMap::new();
        files.insert(
            bundle.root.join("adr").join("0001-doc.md"),
            "---\ntype: ADR\nstatus: Superseded\nsuperseded_by: 0002\n---\n# Doc\n\nBody.\n"
                .to_string(),
        );
        let store = MapStore { files };

        let code = run(&store, &bundle.root);

        assert_ne!(format!("{code:?}"), format!("{:?}", ExitCode::SUCCESS));
    }
}
