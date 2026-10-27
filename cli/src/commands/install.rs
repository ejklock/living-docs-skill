//! `install skills`/`install hooks` verb wrappers (ADR 0060, folding the
//! retired `skill install` and `hooks install` verbs): delegate to
//! `crate::skill_install` and `crate::hooks` respectively.

use crate::hooks::{self, gitattributes};
use crate::output::outln;
use crate::skill_install::{self, Harness};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub(crate) fn run_install_skills(
    harness: Harness,
    project: bool,
    dir: Option<PathBuf>,
    dry_run: bool,
) -> ExitCode {
    skill_install::install(harness, project, dir, dry_run)
}

/// Defaults `--dir` to the current directory when omitted, matching every
/// other subcommand's cwd-relative default. `docs_dir` is the CLI's global
/// `--docs-dir` flag, resolved at install time and pinned into the
/// generated `LIVING_DOCS_BUNDLE=` commands (ADR 0020 scope, resolved once
/// here rather than at hook run time). `quiet` silences the informational
/// `core.hooksPath` warning. Once the hooks are installed, adds the
/// `<docs-dir>/**/index.md merge=union` line to `.gitattributes` (kept once,
/// foreign lines untouched) so branches that each add a record merge without
/// a conflict in the type indexes; a failing hooks step leaves the attributes
/// file as it was, and an attributes file that is a symlink or not a regular
/// file fails the whole install before anything is written.
pub(crate) fn run_install_hooks(
    dir: Option<PathBuf>,
    dry_run: bool,
    docs_dir: &Path,
    quiet: bool,
) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    if let Err(message) = gitattributes::preflight(&project_root) {
        eprintln!("living-docs install hooks: {message}");
        return ExitCode::from(2);
    }
    let code = hooks::install(&project_root, docs_dir, dry_run, quiet);
    if code != ExitCode::SUCCESS {
        return code;
    }
    match add_union_attribute(&project_root, docs_dir, dry_run, quiet) {
        Ok(()) => code,
        Err(message) => {
            eprintln!("living-docs install hooks: {message}");
            ExitCode::from(2)
        }
    }
}

/// Adds the union line, or announces it under `dry_run`. A docs directory
/// that resolves outside the project gets no line and one stderr note.
fn add_union_attribute(
    project_root: &Path,
    docs_dir: &Path,
    dry_run: bool,
    quiet: bool,
) -> Result<(), String> {
    let Some(line) = gitattributes::union_line(project_root, docs_dir) else {
        if !quiet {
            eprintln!(
                "living-docs install hooks: skipped the merge=union line in .gitattributes: --docs-dir {} lies outside {}",
                docs_dir.display(),
                project_root.display()
            );
        }
        return Ok(());
    };
    let path = gitattributes::attributes_path(project_root);
    if dry_run {
        outln!("would add {line} to {}", path.display());
    } else if let Some(written) = gitattributes::add_union_line(project_root, &line)? {
        outln!("wrote {}", written.display());
    }
    Ok(())
}
