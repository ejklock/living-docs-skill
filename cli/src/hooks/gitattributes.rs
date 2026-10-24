//! The `.gitattributes` line that makes git merge a bundle's `index.md`
//! files with its built-in `union` driver, so two branches that each add a
//! record stop conflicting on the derived listing. `install hooks` adds the
//! line once and keeps every other line byte-identical; `uninstall hooks`
//! removes only that line, and removes the file when nothing else was in it.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const ATTRIBUTES_FILE: &str = ".gitattributes";
const INDEX_GLOB: &str = "**/index.md";
const UNION_ATTRIBUTE: &str = "merge=union";

/// The attribute line for `docs_dir` as a pattern relative to `project_root`
/// (`docs/**/index.md merge=union`), or `None` when `docs_dir` resolves
/// outside the project, where no repository pattern can name it. A relative
/// `docs_dir` is taken under `project_root`; both sides are canonicalized
/// (through the nearest existing ancestor when the docs directory is gone), so
/// an absolute path, a `..` path or a symlinked temp root all compare as the
/// directories they name.
pub(crate) fn union_line(project_root: &Path, docs_dir: &Path) -> Option<String> {
    let root = fs::canonicalize(project_root).ok()?;
    let docs = canonical_or_ancestor(&project_root.join(docs_dir))?;
    let segments: Vec<String> = docs
        .strip_prefix(&root)
        .ok()?
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .chain([INDEX_GLOB.to_owned()])
        .collect();
    Some(format!("{} {UNION_ATTRIBUTE}", segments.join("/")))
}

/// Canonical form of `path`, resolving the nearest existing ancestor and
/// re-appending the missing tail; `None` when the path ends in `..`.
fn canonical_or_ancestor(path: &Path) -> Option<PathBuf> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Some(canonical);
    }
    let tail = path.file_name()?;
    Some(canonical_or_ancestor(path.parent()?)?.join(tail))
}

/// Refuses an attributes file that exists but is not a regular file (a
/// symlink, a directory): `Err` names it, and nothing has been written.
pub(crate) fn preflight(project_root: &Path) -> Result<(), String> {
    let path = project_root.join(ATTRIBUTES_FILE);
    match fs::symlink_metadata(&path) {
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("{}: {err}", path.display())),
        Ok(meta) if meta.file_type().is_file() => Ok(()),
        Ok(_) => Err(format!(
            "{} is a symlink or not a regular file; refusing to modify it",
            path.display()
        )),
    }
}

/// Appends `line` to `<project_root>/.gitattributes` unless an identical
/// line is already there, creating the file when absent. The returned path is
/// `Some` only when the file changed.
pub(crate) fn add_union_line(project_root: &Path, line: &str) -> Result<Option<PathBuf>, String> {
    preflight(project_root)?;
    let path = project_root.join(ATTRIBUTES_FILE);
    let existing = read_or_empty(&path)?;
    if existing.lines().any(|present| present == line) {
        return Ok(None);
    }
    fs::write(&path, with_line_appended(&existing, line)).map_err(|err| failure(&path, &err))?;
    Ok(Some(path))
}

/// Removes every line equal to `line` from `<project_root>/.gitattributes`,
/// leaving all other lines (another bundle's included) untouched, and deletes
/// the file when no other line remains. The returned path is `Some` only when
/// the file changed or was removed.
pub(crate) fn remove_union_line(
    project_root: &Path,
    line: &str,
) -> Result<Option<PathBuf>, String> {
    preflight(project_root)?;
    let path = project_root.join(ATTRIBUTES_FILE);
    let existing = read_or_empty(&path)?;
    let kept: String = existing
        .split_inclusive('\n')
        .filter(|present| present.trim_end_matches(['\r', '\n']) != line)
        .collect();
    if kept.len() == existing.len() {
        return Ok(None);
    }
    let outcome = if kept.is_empty() {
        fs::remove_file(&path)
    } else {
        fs::write(&path, kept)
    };
    outcome.map_err(|err| failure(&path, &err))?;
    Ok(Some(path))
}

/// Whether the attributes file currently holds `line`.
pub(crate) fn has_union_line(project_root: &Path, line: &str) -> bool {
    read_or_empty(&project_root.join(ATTRIBUTES_FILE))
        .is_ok_and(|text| text.lines().any(|present| present == line))
}

/// The path `.gitattributes` has under `project_root`, for plan messages.
pub(crate) fn attributes_path(project_root: &Path) -> PathBuf {
    project_root.join(ATTRIBUTES_FILE)
}

fn with_line_appended(existing: &str, line: &str) -> String {
    let ending = if existing.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let separator = if existing.is_empty() || existing.ends_with('\n') {
        ""
    } else {
        ending
    };
    format!("{existing}{separator}{line}{ending}")
}

fn read_or_empty(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(failure(path, &err)),
    }
}

fn failure(path: &Path, err: &io::Error) -> String {
    format!("{}: {err}", path.display())
}
