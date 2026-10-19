//! `check` verb wrapper: resolves the bundle path, compiles the report
//! through `living_docs_core::check::compile`, and renders it as colored
//! text or JSON per the resolved output mode (ADR 0060).

mod render;

use crate::output::{self, ColorMode, OutputMode, Style};
use crate::store::build_store;
use living_docs_core::check::{self, MermaidReport, Report};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_check(
    docs_dir: &Path,
    paths: Vec<PathBuf>,
    require_owner: bool,
    changed_files: &[PathBuf],
    mode: OutputMode,
    color: ColorMode,
    list_advisories: bool,
) -> ExitCode {
    let bundle = check_bundle(docs_dir, paths);
    let Some(report) = compile_scoped(&bundle, require_owner, changed_files) else {
        eprintln!(
            "living-docs check: bundle root not found: {}",
            bundle.display()
        );
        eprintln!(
            "       run from the repo root, or pass the docs directory: living-docs check path/to/docs"
        );
        return ExitCode::from(2);
    };
    if mode.is_json() {
        println!("{}", output::to_json(&report));
    } else {
        render::render_text(&report, color, list_advisories);
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The bundle's report, narrowed to `changed_files` when the caller named
/// any (ADR 0062). An empty list is the unscoped gate, not an empty scope.
fn compile_scoped(bundle: &Path, require_owner: bool, changed_files: &[PathBuf]) -> Option<Report> {
    let report = check::compile(build_store().as_ref(), bundle, require_owner)?;
    if changed_files.is_empty() {
        return Some(report);
    }
    Some(report.scoped_to(changed_files))
}

/// `check --mermaid-only [paths...]`: compiles the mermaid-fence report
/// through `living_docs_core::check::compile_mermaid_only` and renders it as
/// colored text or JSON per the resolved output mode (ADR 0060), the same
/// split `run_check` uses for the full report.
pub(crate) fn run_mermaid_only(paths: &[PathBuf], mode: OutputMode, color: ColorMode) -> ExitCode {
    let report = check::compile_mermaid_only(paths);
    if mode.is_json() {
        println!("{}", output::to_json(&report));
    } else {
        render_mermaid_text(&report, color);
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn render_mermaid_text(report: &MermaidReport, color: ColorMode) {
    for error in &report.errors {
        println!("FAIL {}:{}", error.file, error.line);
        for line in error.message.lines().take(5) {
            println!("    {line}");
        }
    }
    println!();
    if report.ok {
        let line = format!(
            "OK: {} diagram(s) across {} file(s).",
            report.diagrams, report.files
        );
        println!("{}", output::paint(color, Style::Green, &line));
    } else {
        let line = format!(
            "FAIL: {} of {} diagram(s) failed to parse.",
            report.errors.len(),
            report.diagrams
        );
        println!("{}", output::paint(color, Style::Red, &line));
    }
}

/// A positional `[BUNDLE_ROOT]` wins when given; otherwise the global
/// `--docs-dir`, so `--docs-dir X fmt`/`check` operates on `X`, never a
/// hardcoded `docs`.
pub(crate) fn check_bundle(docs_dir: &Path, paths: Vec<PathBuf>) -> PathBuf {
    paths
        .into_iter()
        .next()
        .unwrap_or_else(|| docs_dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_bundle_uses_the_first_path_argument() {
        let bundle = check_bundle(Path::new("/repo/docs"), vec![PathBuf::from("/bundle")]);
        assert_eq!(bundle, PathBuf::from("/bundle"));
    }

    #[test]
    fn check_bundle_falls_back_to_docs_dir_when_no_paths_are_given() {
        let bundle = check_bundle(Path::new("/repo/custom"), Vec::new());
        assert_eq!(bundle, PathBuf::from("/repo/custom"));
    }
}
