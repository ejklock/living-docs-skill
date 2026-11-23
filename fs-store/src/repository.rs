//! The repository reads: whether a repository-relative path exists, judged
//! from the working tree alone without reading a file's contents, and the
//! files that input globs match, read with their contents. Neither follows a
//! path out of the repository root.

use living_docs_core::inputs_hash::{self, Glob};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

/// The nearest ancestor of `bundle` (itself included) that holds a `.git`
/// entry, a directory or a file as in a git worktree, resolved through
/// symlinks. `None` when no ancestor holds one.
pub(crate) fn repository_root(bundle: &Path) -> Option<PathBuf> {
    let resolved = fs::canonicalize(bundle).ok()?;
    resolved
        .ancestors()
        .find(|dir| fs::symlink_metadata(dir.join(".git")).is_ok())
        .map(Path::to_path_buf)
}

/// Whether `relative` names an existing entry below `root`: `Some(true)` for
/// one that resolves inside `root`, `Some(false)` for one that does not exist
/// although everything it could resolve through lies inside `root`. `None`
/// means the path is refused: absolute, holding a `..` segment (both without a
/// filesystem call), or resolving, in whole or up to its nearest existing
/// ancestor, outside `root`, or passing through a symbolic link whose target
/// does not lie inside `root` even though that target cannot be resolved. Only
/// metadata and link targets are read, never contents.
pub(crate) fn entry_exists(root: &Path, relative: &Path) -> Option<bool> {
    let plain = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if !plain {
        return None;
    }
    let joined = root.join(relative);
    let resolved = joined
        .ancestors()
        .find_map(|part| fs::canonicalize(part).ok())?;
    if !resolved.starts_with(root) {
        return None;
    }
    if fs::canonicalize(&joined).is_ok() {
        return Some(true);
    }
    (!leaves_through_a_link(root, relative)).then_some(false)
}

/// Whether a symbolic link on `relative`, walked from `root`, points outside
/// `root`, judged by its target text alone so a missing target gives the same
/// answer as an existing one.
fn leaves_through_a_link(root: &Path, relative: &Path) -> bool {
    let mut at = root.to_path_buf();
    for part in relative.components() {
        at.push(part);
        let Ok(meta) = fs::symlink_metadata(&at) else {
            return false;
        };
        if !meta.is_symlink() {
            continue;
        }
        let Ok(target) = fs::read_link(&at) else {
            return true;
        };
        let base = at.parent().unwrap_or(root);
        if !lies_inside(root, &base.join(target)) {
            return true;
        }
    }
    false
}

/// Whether `target`, read lexically for its `.` and `..` parts, has a nearest
/// existing ancestor that resolves inside `root`.
fn lies_inside(root: &Path, target: &Path) -> bool {
    let mut flat = PathBuf::new();
    for part in target.components() {
        match part {
            Component::ParentDir => {
                flat.pop();
            }
            Component::CurDir => {}
            other => flat.push(other),
        }
    }
    flat.ancestors()
        .find_map(|dir| fs::canonicalize(dir).ok())
        .is_some_and(|found| found.starts_with(root))
}

/// The entries below `root` that `globs` match and `.gitignore` does not
/// exclude, each as its `/`-separated path from `root` with its content: a
/// regular file's bytes, or a symbolic link's target text, never followed. A
/// glob that is absolute, holds `..` or is outside the supported syntax is
/// refused with a store refusal. Every `.git` entry is skipped, and a
/// directory no glob can match below is never entered. Only the repository's
/// own `.gitignore` files apply: no global ignore file, no `.git/info/exclude`
/// and no ignore file above `root`.
pub(crate) fn matching_files(root: &Path, globs: &[String]) -> io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let compiled = inputs_hash::compile(globs)
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error.to_string()))?;
    let mut found = Vec::new();
    for entry in walker(root, compiled.clone()) {
        let entry = entry.map_err(into_io)?;
        let Some(relative) = relative_text(root, entry.path()) else {
            continue;
        };
        let Some(kind) = entry.file_type() else {
            continue;
        };
        if !kind.is_dir() && inputs_hash::matches_any(&compiled, &relative) {
            if let Some(content) = content_of(entry.path(), kind)? {
                found.push((PathBuf::from(relative), content));
            }
        }
    }
    Ok(found)
}

/// A walk of `root` that honors `.gitignore` files only, skips `.git` and
/// enters a directory only when a glob can match below it.
fn walker(root: &Path, globs: Vec<Glob>) -> ignore::Walk {
    let base = root.to_path_buf();
    ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .git_ignore(true)
        .require_git(false)
        .filter_entry(move |entry| {
            let Some(relative) = relative_text(&base, entry.path()) else {
                return false;
            };
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            relative.is_empty()
                || (entry.file_name() != ".git"
                    && (!is_dir || inputs_hash::can_match_below(&globs, &relative)))
        })
        .build()
}

/// The `/`-separated path of `path` below `root`, empty for `root` itself, or
/// `None` when it is not valid text.
fn relative_text(root: &Path, path: &Path) -> Option<String> {
    let below = path.strip_prefix(root).ok()?;
    let parts: Option<Vec<&str>> = below
        .components()
        .map(|part| part.as_os_str().to_str())
        .collect();
    parts.map(|parts| parts.join("/"))
}

/// What a matched entry contributes: a regular file's bytes, a symbolic link's
/// target text, nothing for any other kind.
fn content_of(path: &Path, kind: fs::FileType) -> io::Result<Option<Vec<u8>>> {
    if kind.is_symlink() {
        link_text(path).map(Some)
    } else if kind.is_file() {
        fs::read(path).map(Some)
    } else {
        Ok(None)
    }
}

/// The target of the symbolic link at `path` as git stores it: the raw link
/// text with `/` separators, never resolved.
fn link_text(path: &Path) -> io::Result<Vec<u8>> {
    let target = fs::read_link(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        Ok(target.as_os_str().as_bytes().to_vec())
    }
    #[cfg(not(unix))]
    {
        Ok(target.to_string_lossy().replace('\\', "/").into_bytes())
    }
}

fn into_io(error: ignore::Error) -> io::Error {
    error
        .into_io_error()
        .unwrap_or_else(|| io::Error::other("the repository walk failed"))
}
