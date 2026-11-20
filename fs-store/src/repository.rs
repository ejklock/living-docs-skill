//! The repository probe: whether a repository-relative path exists, judged
//! from the working tree alone. It never reads a file's contents and never
//! follows a path out of the repository root.

use std::fs;
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
