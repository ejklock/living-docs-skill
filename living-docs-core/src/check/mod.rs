//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format, directory-index membership and drift, bundle-root reachability,
//! supersede-chain integrity, duplicate record numbers, local link/image validity via `pulldown-cmark`, unfilled placeholders, and
//! ```mermaid``` fence validation. Record liveness (`stale-proposed` only) and supersede cycles are advisory.
//!
//! Every record's content (`records`, `links`) is read through `DocStore::read`, so `check` validates whichever backend
//! `compile` is given. `index.md`/`log.md` sit outside the record domain by design; `check::graph` reads them from disk.

mod callout;
mod cancellable;
pub(crate) mod canonical;
mod dependencies;
mod duplicate_numbers;
mod frontmatter_advice;
mod graph;
pub(crate) use graph::{normpath, reachable_index_set};
mod decision_table;
mod index_drift;
pub mod kinds;
mod line_endings;
mod link_style;
pub(crate) mod links;
mod liveness;
mod mermaid;
mod moved_source;
mod path_missing;
mod placeholder;
mod read_once;
mod records;
mod report;
mod sections;
mod size;
mod stale_view;
pub(crate) mod state_machine;
mod supersede_cycle;
mod supersede_links;
mod unparseable_frontmatter;

use crate::doc_type::{self, Identity};
use crate::store::DocStore;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::ExitCode;

use cancellable::run_all_checks;
pub use cancellable::{compile_cancellable, Checked};
pub use mermaid::{MermaidError, MermaidReport};
pub(crate) use records::is_retired_record;
pub use report::{Advisory, Report, Violation};

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences under `paths`, skipping every other
/// invariant, without printing or choosing an exit code. See `mermaid::compile`.
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

/// The invariants [`compile`] validates, minus the unfilled-placeholder pass, returned as a plain violation list
/// rather than printed — for a caller that must reject a structurally invalid record. The placeholder pass is
/// deliberately excluded here: a freshly created record legitimately still carries its `{{PLACEHOLDER}}` slots until
/// the author fills them in. Unfilled placeholders are caught by the full `check` command, which runs the pass.
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
/// The comparison is a plain path equality, not a filesystem lookup, so it only stays correct because every [`DocStore`]
/// enumerates a bundle's paths rooted at the same `bundle` it was given (see [`collect_md_files`]) — `read_dir` never
/// resolves symlinks, so this holds even when the caller reaches the bundle through one. Reach for `canonicalize` here
/// instead and this pure predicate starts touching disk, breaking every `MapStore` fixture whose paths never exist on disk.
pub(crate) fn is_bundle_singleton(bundle: &Path, path: &Path) -> bool {
    doc_type::DOC_TYPES.iter().any(|spec| match spec.identity {
        Identity::Singleton { file } => path == bundle.join(file),
        Identity::Numbered { .. } | Identity::Named { .. } | Identity::Nested { .. } => false,
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
            anchors: Vec::new(),
        });
    }

    /// Advises once for a group of records, anchored on every member so a
    /// scope holding any one of them keeps it. The members are not serialized.
    pub(crate) fn advise_group(
        &mut self,
        members: &[PathBuf],
        kind: &str,
        message: impl Into<String>,
    ) {
        let Some(first) = members.first() else {
            return;
        };
        self.advisories.push(Advisory {
            file: first.display().to_string(),
            kind: kind.to_owned(),
            message: message.into(),
            anchors: members.iter().map(|m| m.display().to_string()).collect(),
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
