//! Native core of `living-docs check` — ports `skills/living-docs/scripts/lint-docs.sh`.
//!
//! Covers the mechanical invariants: OKF frontmatter/type, index-format,
//! directory-index membership, bundle-root reachability, supersede-chain
//! integrity, local link/image validity via `pulldown-cmark`, and (S6)
//! ```mermaid``` fence validation via the pinned mermaid-cli Docker image.

mod graph;
mod links;
mod mermaid;
mod records;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// `check --mermaid-only [paths...]` — validates ONLY the mermaid fences under
/// `paths`, skipping every other invariant. See `mermaid::run_mermaid_only`.
pub fn run_mermaid_only(paths: &[PathBuf]) -> ExitCode {
    mermaid::run_mermaid_only(paths)
}

pub fn run(bundle: &Path) -> ExitCode {
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

    let all_md = collect_md_files(bundle);
    let root_index = bundle.join("index.md");
    let mut reporter = Reporter::new();

    println!("Living Docs lint — bundle: {}", bundle.display());
    println!();

    if !root_index.is_file() {
        reporter.report(&root_index, "missing bundle-root index.md (invariant 3)");
    }

    records::check_frontmatter_and_format(&all_md, &root_index, &mut reporter);
    graph::check_directory_membership(bundle, &all_md, &mut reporter);
    graph::check_reachability(bundle, &root_index, &all_md, &mut reporter);
    links::check_links(bundle, &all_md, &mut reporter);
    records::check_supersede_chain(&all_md, &mut reporter);

    if let Some(code) = mermaid::check_bundle(&all_md, &mut reporter) {
        return code;
    }

    reporter.finish(all_md.len())
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
/// `report()` and the verdict block of `lint-docs.sh`.
pub(crate) struct Reporter {
    violations: Vec<(String, String)>,
}

impl Reporter {
    fn new() -> Self {
        Self {
            violations: Vec::new(),
        }
    }

    pub(crate) fn report(&mut self, file: &Path, message: impl Into<String>) {
        self.violations
            .push((file.display().to_string(), message.into()));
    }

    fn finish(self, doc_count: usize) -> ExitCode {
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
}
