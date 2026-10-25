//! `DocStore` adapter over `.md` files on disk, rooted at one bundle. Mirrors
//! `living_docs_core::check`'s doc-enumeration semantics: recursive walk,
//! `.md` files only, sorted, lenient on unreadable subdirectories.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use living_docs_core::store::DocStore;

/// `DocStore` implementation backed by the local filesystem, built for one
/// bundle directory.
#[derive(Debug, Clone)]
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    /// Builds a store for the bundle at `bundle`. The root is resolved on
    /// every call rather than here, so a bundle that does not exist yet
    /// still works.
    pub fn rooted(bundle: impl Into<PathBuf>) -> Self {
        Self {
            root: bundle.into(),
        }
    }
}

impl DocStore for FsStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut found = Vec::new();
        collect_md_files(root, &mut found);
        found.sort();
        Ok(found)
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.confine(path)?;
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.confine(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.confine(from)?;
        self.confine(to)?;
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(from, to)
    }
}

impl FsStore {
    /// Refuses `path` unless its resolved location lies inside the resolved
    /// bundle root, with a `PermissionDenied` error naming it. A path through
    /// a dangling link is refused as such, since it has no location to judge.
    fn confine(&self, path: &Path) -> io::Result<()> {
        let resolved = resolve(path);
        let inside = match (&resolved, resolve(&self.root)) {
            (Ok(target), Ok(root)) => target.starts_with(root),
            _ => false,
        };
        if inside {
            return Ok(());
        }
        let message = if is_dangling(path) {
            format!(
                "{} is a dangling link; a dangling link cannot be judged",
                path.display()
            )
        } else {
            format!(
                "{} resolves outside the bundle {}; a record must lie inside the bundle",
                path.display(),
                self.root.display()
            )
        };
        Err(io::Error::new(io::ErrorKind::PermissionDenied, message))
    }
}

/// True when `path`, or a directory above it, is a symlink whose target does
/// not exist.
fn is_dangling(path: &Path) -> bool {
    path.ancestors().any(|part| {
        fs::symlink_metadata(part).is_ok_and(|meta| meta.is_symlink())
            && fs::canonicalize(part).is_err()
    })
}

/// Resolves `path` through every symlink on its existing components. A path
/// that does not exist yet is its nearest existing ancestor, resolved, plus
/// the components below it; a `..` below that ancestor and a dangling link
/// cannot be judged, so they are errors.
fn resolve(path: &Path) -> io::Result<PathBuf> {
    let mut tail = Vec::new();
    let mut head = path;
    loop {
        let probe = if head.as_os_str().is_empty() {
            Path::new(".")
        } else {
            head
        };
        match fs::canonicalize(probe) {
            Ok(base) => return Ok(tail.iter().rev().fold(base, |acc, name| acc.join(name))),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                if fs::symlink_metadata(probe).is_ok() {
                    return Err(error);
                }
                let (Some(name), Some(parent)) = (head.file_name(), head.parent()) else {
                    return Err(error);
                };
                tail.push(name);
                head = parent;
            }
            Err(error) => return Err(error),
        }
    }
}

fn collect_md_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            collect_md_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests;
