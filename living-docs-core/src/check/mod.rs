//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format, directory-index membership and drift,
//! bundle-root reachability, supersede-chain integrity, duplicate record numbers, local link/image validity via `pulldown-cmark`,
//! unfilled placeholders, and ```mermaid``` fence validation. Record liveness (`stale-proposed` only) is advisory.
//!
//! Every record's content (`records`, `links`) is read through `DocStore::read`,
//! so `check` validates whichever backend `compile` is given. `index.md`/`log.md`
//! sit outside the record domain by design; `check::graph` reads them from disk.

mod callout;
pub(crate) mod canonical;
mod duplicate_numbers;
mod graph;
pub(crate) use graph::{normpath, reachable_index_set};
mod index_drift;
pub mod kinds;
mod line_endings;
pub(crate) mod links;
mod liveness;
mod mermaid;
mod moved_source;
mod placeholder;
mod records;
mod report;
mod sections;
mod size;
mod supersede_links;

use crate::doc_type::{self, Identity};
use crate::line_ending::LineEndingStore;
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::ExitCode;

pub use mermaid::{MermaidError, MermaidReport};
pub(crate) use records::is_retired_record;
pub use report::{Advisory, Report, Violation};

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences
/// under `paths`, skipping every other invariant, without printing or
/// choosing an exit code. See `mermaid::compile`.
pub fn compile_mermaid_only(paths: &[PathBuf]) -> MermaidReport {
    mermaid::compile(paths)
}

/// Compiles `check`'s full report — every invariant plus the placeholder
/// pass — without printing or choosing an exit code. `None` when `bundle` is
/// not a directory (the CLI front turns that into its own usage error).
pub fn compile(store: &dyn DocStore, bundle: &Path, require_owner: bool) -> Option<Report> {
    if !bundle.is_dir() {
        return None;
    }
    let mut reporter = Reporter::new();
    let doc_count = run_all_checks(store, bundle, &mut reporter, require_owner, true);
    Some(reporter.into_report(bundle, doc_count))
}

/// Every invariant `check` validates, without the `bundle.is_dir()` guard,
/// header, or verdict rendering — shared by [`compile`] (the full
/// [`Report`]) and [`check_violations`] (the raw list, for a caller like
/// `db_store::DbDocStore::write_checked` that gates a write on the same
/// invariants). Records are judged in LF form, mixed endings flagged from `raw`.
fn run_all_checks(
    raw: &dyn DocStore,
    bundle: &Path,
    reporter: &mut Reporter,
    require_owner: bool,
    check_placeholders: bool,
) -> usize {
    let store: &dyn DocStore = &LineEndingStore::new(raw, true);
    let all_md = store.list(bundle).unwrap_or_default();
    line_endings::check_line_endings(raw, &all_md, reporter);
    let root_index = bundle.join("index.md");
    if !root_index.is_file() {
        reporter.report(&root_index, "missing bundle-root index.md (invariant 3)");
    }
    let doc_count = all_md.len();
    let all_md = judge_records(store, all_md, &root_index, reporter);
    graph::check_directory_membership(bundle, &all_md, reporter);
    graph::check_reachability(bundle, &root_index, &all_md, reporter);
    index_drift::check_index_drift(raw, bundle, reporter);
    links::check_links(store, bundle, &all_md, reporter);
    records::check_supersede_chain(store, &all_md, reporter);
    supersede_links::check_supersede_links(store, bundle, &all_md, reporter);
    duplicate_numbers::check_duplicate_numbers(bundle, &all_md, reporter);
    callout::check_callouts(store, &all_md, reporter);
    moved_source::check_moved_source(store, bundle, &all_md, reporter);
    records::check_owner_requirement(store, &all_md, require_owner, reporter);
    records::check_heading_matches_title(store, &all_md, reporter);
    canonical::check_canonical_frontmatter(store, bundle, &all_md, reporter);
    mermaid::check_bundle(&all_md, reporter);
    size::check_body_size(store, &all_md, reporter);
    sections::check_required_sections(store, &all_md, reporter);
    if check_placeholders {
        placeholder::check_placeholders(store, &all_md, reporter);
    }
    liveness::check_liveness(store, bundle, &all_md, reporter);
    doc_count
}

/// Runs the per-record frontmatter pass and returns `all_md` without the
/// records the store refused, which every later check leaves out.
fn judge_records(
    store: &dyn DocStore,
    all_md: Vec<PathBuf>,
    root_index: &Path,
    reporter: &mut Reporter,
) -> Vec<PathBuf> {
    let refused = records::check_frontmatter_and_format(store, &all_md, root_index, reporter);
    all_md
        .into_iter()
        .filter(|f| !refused.contains(f))
        .collect()
}

/// The invariants [`run`] validates, minus the unfilled-placeholder pass, returned as a plain violation list rather
/// than printed — the mechanism a per-write gate (e.g. `db_store::DbDocStore::write_checked`) uses to reject a
/// structurally invalid record. The placeholder pass is deliberately excluded here: db-mode authoring is
/// create-then-edit, so a freshly created record legitimately still carries its `{{PLACEHOLDER}}` slots until the
/// author fills them in the edit view. Unfilled placeholders are caught at the commit boundary by the `check`
/// command (which runs the pass), not by the per-write gate.
pub fn check_violations(store: &dyn DocStore, bundle: &Path) -> Vec<(String, String)> {
    let mut reporter = Reporter::new();
    run_all_checks(store, bundle, &mut reporter, false, false);
    reporter
        .violations
        .into_iter()
        .map(|violation| (violation.file, violation.message))
        .collect()
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

/// True when `path` is exactly the bundle-root file of some registry
/// [`Identity::Singleton`] row — the single place the check layer learns
/// what a singleton is, so a second singleton row is handled without an
/// edit here.
///
/// The comparison is a plain path equality, not a filesystem lookup, so it
/// only stays correct because every [`DocStore`] enumerates a bundle's paths
/// rooted at the same `bundle` it was given (see [`collect_md_files`]) —
/// `read_dir` never resolves symlinks, so this holds even when the caller
/// reaches the bundle through one. Reach for `canonicalize` here instead and
/// this pure predicate starts touching disk, breaking every `MapStore`
/// fixture whose paths never exist on disk.
pub(crate) fn is_bundle_singleton(bundle: &Path, path: &Path) -> bool {
    doc_type::DOC_TYPES.iter().any(|spec| match spec.identity {
        Identity::Singleton { file } => path == bundle.join(file),
        Identity::Numbered { .. } | Identity::Named { .. } => false,
    })
}

/// Collects violations and advisories as `run_all_checks` walks the bundle,
/// mirroring `report()`/`advise()` of `lint-docs.sh`.
pub(crate) struct Reporter {
    violations: Vec<Violation>,
    advisories: Vec<Advisory>,
}

impl Reporter {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
            advisories: Vec::new(),
        }
    }

    pub(crate) fn report(&mut self, file: &Path, message: impl Into<String>) {
        self.violations.push(Violation {
            file: file.display().to_string(),
            message: message.into(),
            anchors: Vec::new(),
        });
    }

    /// Reports one violation for a group of records, anchored on every member
    /// so a scope holding any one of them keeps it. The members are not
    /// serialized.
    pub(crate) fn report_group(&mut self, members: &[PathBuf], message: impl Into<String>) {
        let Some(first) = members.first() else {
            return;
        };
        self.violations.push(Violation {
            file: first.display().to_string(),
            message: message.into(),
            anchors: members.iter().map(|m| m.display().to_string()).collect(),
        });
    }

    pub(crate) fn advise(&mut self, file: &Path, kind: &str, message: impl Into<String>) {
        self.advisories.push(Advisory {
            file: file.display().to_string(),
            kind: kind.to_owned(),
            message: message.into(),
        });
    }

    fn into_report(self, bundle: &Path, docs: usize) -> Report {
        Report {
            bundle: bundle.display().to_string(),
            docs,
            ok: self.violations.is_empty(),
            violations: self.violations,
            advisories: self.advisories,
        }
    }

    #[cfg(test)]
    fn into_violations(self) -> Vec<(String, String)> {
        self.violations
            .into_iter()
            .map(|violation| (violation.file, violation.message))
            .collect()
    }

    #[cfg(test)]
    fn finish(&self) -> ExitCode {
        if self.violations.is_empty() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod scope_tests;
#[cfg(test)]
mod tests;
