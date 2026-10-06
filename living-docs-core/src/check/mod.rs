//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format,
//! directory-index membership, bundle-root reachability, supersede-chain
//! integrity, local link/image validity via `pulldown-cmark`, unfilled
//! placeholders, and ```mermaid``` fence validation (ADR 0013). Record
//! liveness (ADR 0049, `stale-proposed` only) is advisory.
//!
//! Every record's content (`records`, `links`) is read through
//! `DocStore::read`, so `check` validates whichever backend `compile` is
//! given. `index.md`/`log.md` are excluded from the record domain by design
//! (never synced to `db-store`); `check::graph` reads them straight from
//! disk.

mod callout;
pub(crate) mod canonical;
mod graph;
pub(crate) mod links;
mod liveness;
mod mermaid;
mod moved_source;
mod placeholder;
mod records;
mod sections;
mod size;

use crate::doc_type::{self, Identity};
use crate::store::DocStore;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::process::ExitCode;

pub use mermaid::{MermaidError, MermaidReport};

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences
/// under `paths`, skipping every other invariant, without printing or
/// choosing an exit code, so the CLI front can render it as colored text or
/// JSON (ADR 0060). See `mermaid::compile`.
pub fn compile_mermaid_only(paths: &[PathBuf]) -> MermaidReport {
    mermaid::compile(paths)
}

/// Compiles `check`'s full report — every invariant plus the placeholder
/// pass — without printing or choosing an exit code, so the CLI front can
/// render it as colored text or JSON (ADR 0060). `None` when `bundle` is not
/// a directory (the CLI front turns that into its own usage error).
pub fn compile(store: &dyn DocStore, bundle: &Path, require_owner: bool) -> Option<Report> {
    if !bundle.is_dir() {
        return None;
    }
    let mut reporter = Reporter::new();
    let doc_count = run_all_checks(store, bundle, &mut reporter, require_owner, true);
    Some(reporter.into_report(bundle, doc_count))
}

/// Every invariant `check` validates, without the surrounding
/// `bundle.is_dir()` guard, header, or verdict rendering — shared by
/// [`compile`] (which builds the full [`Report`]) and [`check_violations`]
/// (which returns the raw list, for a caller like
/// `db_store::DbDocStore::write_checked` that gates a write on the same
/// invariants without printing anything). Returns the number of docs
/// `store` enumerated under `bundle`.
fn run_all_checks(
    store: &dyn DocStore,
    bundle: &Path,
    reporter: &mut Reporter,
    require_owner: bool,
    check_placeholders: bool,
) -> usize {
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

    all_md.len()
}

/// The invariants [`run`] validates, minus the unfilled-placeholder pass,
/// returned as a plain violation list rather than printed — the mechanism a
/// per-write gate (e.g. `db_store::DbDocStore::write_checked`) uses to reject a
/// structurally invalid record. The placeholder pass is deliberately excluded
/// here: db-mode authoring is create-then-edit, so a freshly created record
/// legitimately still carries its `{{PLACEHOLDER}}` slots until the author
/// fills them in the edit view. Unfilled placeholders are caught at the commit
/// boundary by the `check` command (which runs the pass), not by the per-write
/// gate.
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

/// A hard invariant failure: `check`'s exit code goes non-zero when any
/// exist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Violation {
    pub file: String,
    pub message: String,
}

/// A soft finding (issue 0009): printed alongside violations but never
/// affects the exit code. `kind` is the advisory's category — `size`,
/// `owner`, `liveness`, or `moved-source` — so a JSON consumer can filter by
/// it without parsing `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Advisory {
    pub file: String,
    pub kind: String,
    pub message: String,
}

/// `check`'s full report (ADR 0060): every invariant's violations and
/// advisories, the bundle enumerated and its doc count, and `ok` (no
/// violations) precomputed so a renderer never has to re-derive it.
#[derive(Debug, Serialize)]
pub struct Report {
    pub bundle: String,
    pub docs: usize,
    pub violations: Vec<Violation>,
    pub advisories: Vec<Advisory>,
    pub ok: bool,
}

impl Report {
    /// The same report with every finding not anchored to one of `paths`
    /// dropped, and `ok` recomputed from what remains (ADR 0062). Every
    /// invariant still ran over the whole bundle — this narrows what is
    /// reported, never what was computed — so a brownfield bundle can gate a
    /// commit on the records it touched while legacy debt elsewhere is paid
    /// down. An empty `paths` narrows to nothing and reports nothing.
    #[must_use]
    pub fn scoped_to(self, paths: &[PathBuf]) -> Self {
        let scope: Vec<String> = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        let violations: Vec<Violation> = self
            .violations
            .into_iter()
            .filter(|violation| anchors_in(&violation.file, &scope))
            .collect();
        Self {
            ok: violations.is_empty(),
            violations,
            advisories: self
                .advisories
                .into_iter()
                .filter(|advisory| anchors_in(&advisory.file, &scope))
                .collect(),
            ..self
        }
    }
}

/// Whether `file` names the same record as one of `scope`. The two come from
/// different hands — the report's paths are the store's enumeration, the
/// scope's are whatever git or the caller printed — so one is allowed to be
/// the other's suffix: `docs/adr/0001-x.md` matches an absolute
/// `/repo/docs/adr/0001-x.md` and vice versa.
fn anchors_in(file: &str, scope: &[String]) -> bool {
    scope.iter().any(|candidate| {
        file == candidate
            || file.ends_with(&format!("/{candidate}"))
            || candidate.ends_with(&format!("/{file}"))
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
