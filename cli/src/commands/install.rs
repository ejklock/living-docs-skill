//! `install skills`/`install hooks`/`install mermaid` verb wrappers: skills
//! and hooks delegate to `crate::skill_install` and `crate::hooks` and write
//! into a project; mermaid delegates to `crate::mermaid_asset` and writes
//! into the user data directory, never the project.

use crate::hooks::{self, gitattributes};
use crate::mermaid_asset;
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

/// Places the pinned mermaid file in the user data directory: downloads it
/// with `curl` and keeps it only when its SHA-256 is the pinned one. Under
/// `dry_run` it prints the URL and destination and touches nothing.
pub(crate) fn run_install_mermaid(dry_run: bool) -> ExitCode {
    let dest = match mermaid_asset::installed_path() {
        Ok(dest) => dest,
        Err(message) => return mermaid_failure(&message),
    };
    if dry_run {
        outln!(
            "would download {} to {}",
            mermaid_asset::URL,
            dest.display()
        );
        return ExitCode::SUCCESS;
    }
    match mermaid_asset::download_verified(
        Path::new("curl"),
        mermaid_asset::URL,
        mermaid_asset::SHA256,
        &dest,
    ) {
        Ok(()) => {
            outln!("mermaid {} -> {}", mermaid_asset::VERSION, dest.display());
            ExitCode::SUCCESS
        }
        Err(message) => mermaid_failure(&message),
    }
}

fn mermaid_failure(message: &str) -> ExitCode {
    eprintln!("living-docs manage install mermaid: {message}");
    ExitCode::from(2)
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
        eprintln!("living-docs manage install hooks: {message}");
        return ExitCode::from(2);
    }
    let code = hooks::install(&project_root, docs_dir, dry_run, quiet);
    if code != ExitCode::SUCCESS {
        return code;
    }
    match add_union_attribute(&project_root, docs_dir, dry_run, quiet) {
        Ok(()) => code,
        Err(message) => {
            eprintln!("living-docs manage install hooks: {message}");
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
                "living-docs manage install hooks: skipped the merge=union line in .gitattributes: --docs-dir {} lies outside {}",
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
