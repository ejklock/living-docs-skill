//! `uninstall hooks` verb wrapper (folding the retired `hooks uninstall`
//! verb): delegates to `crate::hooks`.

use crate::hooks::{self, gitattributes};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Defaults `--dir` to the current directory, mirroring `install hooks`.
/// After the hooks are removed, also removes the one `merge=union` line
/// `install hooks` added to `.gitattributes` for the bundle named by the
/// global `--docs-dir` flag (default `docs`), computed the way install
/// computes it, so another bundle's line and foreign lines stay; the file goes
/// when nothing else was in it. An attributes file that is a symlink or not a
/// regular file fails the run before anything is removed.
pub(crate) fn run_uninstall_hooks(
    dir: Option<PathBuf>,
    dry_run: bool,
    docs_dir: &Path,
) -> ExitCode {
    let project_root = dir.unwrap_or_else(|| PathBuf::from("."));
    if let Err(message) = gitattributes::preflight(&project_root) {
        eprintln!("living-docs uninstall hooks: {message}");
        return ExitCode::from(2);
    }
    let code = hooks::uninstall(&project_root, dry_run);
    if code != ExitCode::SUCCESS {
        return code;
    }
    match remove_union_attribute(&project_root, docs_dir, dry_run) {
        Ok(()) => code,
        Err(message) => {
            eprintln!("living-docs uninstall hooks: {message}");
            ExitCode::from(2)
        }
    }
}

/// Removes this bundle's union line, or announces its removal under `dry_run`.
fn remove_union_attribute(
    project_root: &Path,
    docs_dir: &Path,
    dry_run: bool,
) -> Result<(), String> {
    let Some(line) = gitattributes::union_line(project_root, docs_dir) else {
        return Ok(());
    };
    if dry_run {
        if gitattributes::has_union_line(project_root, &line) {
            let path = gitattributes::attributes_path(project_root);
            println!("would remove {line} from {}", path.display());
        }
    } else if let Some(path) = gitattributes::remove_union_line(project_root, &line)? {
        println!("stripped {}", path.display());
    }
    Ok(())
}
