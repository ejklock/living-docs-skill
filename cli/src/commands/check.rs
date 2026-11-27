//! `check` verb wrapper: resolves the bundle path, compiles the report
//! through `living_docs_core::check::compile`, and renders it as colored
//! text or JSON per the resolved output mode.

mod render;

use crate::output::{self, ColorMode, OutputMode, Style};
use crate::store::build_store;
use living_docs_core::check::{self, MermaidReport, Report};
use living_docs_core::commands::missing_bundle;
use living_docs_core::doc_type::{self, Identity};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// What the caller asked `check` to say beyond the report itself.
pub(crate) struct Verbosity {
    pub(crate) list_advisories: bool,
    pub(crate) quiet: bool,
}

/// The paths `check` narrows its report to: records, and where to read the
/// repository paths that only keep a stale view in scope.
pub(crate) struct Changes<'a> {
    pub(crate) files: &'a [PathBuf],
    pub(crate) inputs_from: Option<&'a str>,
}

pub(crate) fn run_check(
    docs_dir: &Path,
    paths: Vec<PathBuf>,
    require_owner: bool,
    changes: &Changes,
    mode: OutputMode,
    color: ColorMode,
    verbosity: &Verbosity,
) -> ExitCode {
    let inputs = match read_inputs(changes.inputs_from) {
        Ok(inputs) => inputs,
        Err(error) => {
            eprintln!("living-docs check: cannot read --changed-inputs-from: {error}");
            return ExitCode::from(2);
        }
    };
    let (bundle, scope) = resolve_target(check_bundle(docs_dir, paths), docs_dir, changes.files);
    let Some(report) = compile_scoped(&bundle, require_owner, &scope, inputs.as_deref()) else {
        return exit_bundle_root_not_found(&bundle);
    };
    if mode.is_json() {
        output::outln!("{}", output::to_json(&report));
    } else {
        render::render_text(&report, color, verbosity.list_advisories);
    }
    if verbosity.list_advisories {
        note_missing_doc_gate(&bundle, verbosity.quiet);
    }
    if report.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Tells the user where to run `check` from, and exits with the usage code.
fn exit_bundle_root_not_found(bundle: &Path) -> ExitCode {
    eprintln!("living-docs check: {}", missing_bundle::message(bundle));
    eprintln!(
        "       run from the repo root, or pass the docs directory: living-docs check path/to/docs"
    );
    ExitCode::from(2)
}

/// Suggests `manage install hooks` on stderr when the repository holding `bundle`
/// has no pre-commit doc-gate; `run_check` asks only under `--advisories`.
/// Silent outside a git repository and under `--quiet`.
fn note_missing_doc_gate(bundle: &Path, quiet: bool) {
    if crate::doc_gate::doc_gate_armed(bundle) == Some(false) {
        output::note(
            quiet,
            "living-docs check: no pre-commit doc-gate is armed in this repository; run `living-docs manage install hooks` to enforce it at commit",
        );
    }
}

/// The bundle to check and the files its report is narrowed to. A `target`
/// that is a record file resolves, symlinks followed, to its enclosing bundle
/// and scopes the report to that record on top of any `changed_files`; any
/// other target is the bundle itself, scoped to `changed_files` alone.
fn resolve_target(
    target: PathBuf,
    docs_dir: &Path,
    changed_files: &[PathBuf],
) -> (PathBuf, Vec<PathBuf>) {
    let mut scope = changed_files.to_vec();
    let Some((bundle, record)) = enclosing_bundle(&target, docs_dir) else {
        return (target, scope);
    };
    scope.push(record);
    (bundle, scope)
}

/// The bundle holding the record file `target` and the record's path inside
/// it, both relative to the working directory when they lie under it.
/// `None` when `target` is not an existing file or [`bundle_root_of`] finds
/// no bundle for it.
fn enclosing_bundle(target: &Path, docs_dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let record = target.canonicalize().ok().filter(|path| path.is_file())?;
    let root = bundle_root_of(&record, docs_dir)?;
    let cwd = Path::new(".").canonicalize().ok()?;
    let shown = root.strip_prefix(&cwd).unwrap_or(&root);
    let bundle = if shown.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        shown.to_path_buf()
    };
    let inside = record.strip_prefix(&root).ok()?;
    Some((bundle.clone(), bundle.join(inside)))
}

/// The bundle root of the canonical record path `record`: the `--docs-dir`
/// directory when the record lies inside it; otherwise the parent of the
/// nearest enclosing registered type directory, so a nested session directory
/// still resolves to its bundle, but only when that parent or the type
/// directory itself holds an `index.md`; otherwise the record's own directory when
/// the record is a bundle-root singleton file. `None` for anything else.
pub(crate) fn bundle_root_of(record: &Path, docs_dir: &Path) -> Option<PathBuf> {
    if let Ok(docs) = docs_dir.canonicalize() {
        if record.starts_with(&docs) {
            return Some(docs);
        }
    }
    let typed = record.ancestors().skip(1).find(|dir| {
        dir.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| doc_type::spec_for_dir(name).is_some())
    });
    let holds_index = |dir: &Path| dir.join("index.md").is_file();
    let indexed_parent = typed
        .and_then(Path::parent)
        .filter(|parent| holds_index(parent) || typed.is_some_and(holds_index));
    if let Some(parent) = indexed_parent {
        return Some(parent.to_path_buf());
    }
    let name = record.file_name()?.to_str()?;
    let is_singleton = doc_type::DOC_TYPES
        .iter()
        .any(|spec| matches!(spec.identity, Identity::Singleton { file } if file == name));
    if is_singleton {
        return record.parent().map(Path::to_path_buf);
    }
    None
}

/// The NUL-separated paths in `source`, a file or `-` for standard input, as
/// text; `None` when no source was named. An empty entry is dropped.
fn read_inputs(source: Option<&str>) -> std::io::Result<Option<Vec<String>>> {
    let Some(source) = source else {
        return Ok(None);
    };
    let mut bytes = Vec::new();
    if source == "-" {
        std::io::stdin().read_to_end(&mut bytes)?;
    } else {
        bytes = std::fs::read(source)?;
    }
    let paths = bytes
        .split(|byte| *byte == 0)
        .filter(|path| !path.is_empty())
        .map(|path| String::from_utf8_lossy(path).into_owned())
        .collect();
    Ok(Some(paths))
}

/// The bundle's report, narrowed to `changed_files` and to the stale views
/// `inputs` reach when the caller named either. With neither it is the
/// unscoped gate, not an empty scope; an `inputs` source naming no path
/// narrows to nothing.
fn compile_scoped(
    bundle: &Path,
    require_owner: bool,
    changed_files: &[PathBuf],
    inputs: Option<&[String]>,
) -> Option<Report> {
    let report = check::compile(build_store(bundle).as_ref(), bundle, require_owner)?;
    if changed_files.is_empty() && inputs.is_none() {
        return Some(report);
    }
    Some(report.scoped_to_changes(changed_files, inputs.unwrap_or_default()))
}

/// `check --mermaid-only [paths...]`: compiles the mermaid-fence report
/// through `living_docs_core::check::compile_mermaid_only` and renders it as
/// colored text or JSON per the resolved output mode, the same
/// split `run_check` uses for the full report.
pub(crate) fn run_mermaid_only(paths: &[PathBuf], mode: OutputMode, color: ColorMode) -> ExitCode {
    let report = check::compile_mermaid_only(paths);
    if mode.is_json() {
        output::outln!("{}", output::to_json(&report));
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
        output::outln!("FAIL {}:{}", error.file, error.line);
        for line in error.message.lines().take(5) {
            output::outln!("    {line}");
        }
    }
    output::outln!();
    if report.ok {
        let line = format!(
            "OK: {} diagram(s) across {} file(s).",
            report.diagrams, report.files
        );
        output::outln!("{}", output::paint(color, Style::Green, &line));
    } else {
        let line = format!(
            "FAIL: {} of {} diagram(s) failed to parse.",
            report.errors.len(),
            report.diagrams
        );
        output::outln!("{}", output::paint(color, Style::Red, &line));
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
