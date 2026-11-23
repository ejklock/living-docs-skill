//! Shared `#[cfg(test)]` fixtures: in-memory [`crate::store::DocStore`]
//! doubles backed by a `BTreeMap`, with no filesystem I/O, so every unit test
//! reuses one store instead of carrying its own copy.

use crate::store::{DocStore, RepositoryFiles};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct MapStore {
    pub(crate) files: BTreeMap<PathBuf, String>,
}

impl MapStore {
    /// A store and its `all_md` listing seeded from `(path, contents)`
    /// pairs — the fixture shape `check`'s per-file tests build over and
    /// over, so a scenario is one call instead of a `BTreeMap` built by hand.
    pub(crate) fn seeded(seed: &[(&str, &str)]) -> (Self, Vec<PathBuf>) {
        let files: BTreeMap<PathBuf, String> = seed
            .iter()
            .map(|(path, contents)| (PathBuf::from(*path), (*contents).to_string()))
            .collect();
        let all_md = files.keys().cloned().collect();
        (Self { files }, all_md)
    }
}

impl DocStore for MapStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
        Ok(())
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Ok(())
    }

    fn repository_files(&self, globs: &[String]) -> Option<RepositoryFiles> {
        let compiled = match crate::inputs_hash::compile(globs) {
            Ok(compiled) => compiled,
            Err(error) => {
                return Some(Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    error.to_string(),
                )))
            }
        };
        let matched = self
            .files
            .iter()
            .filter(|(path, _)| crate::inputs_hash::matches_any(&compiled, &path.to_string_lossy()))
            .map(|(path, body)| (path.clone(), body.clone().into_bytes()))
            .collect();
        Some(Ok(matched))
    }
}

/// A [`MapStore`] that keeps what it is told to write, for the command tests
/// that assert on a record's contents after a verb ran. Mutation lives
/// behind a `RefCell` so the `DocStore` shared-reference contract holds.
pub(crate) struct WritableMapStore {
    files: RefCell<BTreeMap<PathBuf, String>>,
}

impl WritableMapStore {
    /// An empty store, for a test that scaffolds into a bundle with
    /// nothing in it yet.
    pub(crate) fn new() -> Self {
        Self::seeded(&[])
    }

    pub(crate) fn seeded(seed: &[(&str, &str)]) -> Self {
        Self {
            files: RefCell::new(
                seed.iter()
                    .map(|(path, contents)| (PathBuf::from(path), (*contents).to_string()))
                    .collect(),
            ),
        }
    }

    /// What the store holds at `path` — the empty string when it holds
    /// nothing, so a test asserting on absence reads as plainly as one
    /// asserting on content.
    pub(crate) fn contents(&self, path: &str) -> String {
        self.files
            .borrow()
            .get(&PathBuf::from(path))
            .cloned()
            .unwrap_or_default()
    }
}

impl DocStore for WritableMapStore {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(self
            .files
            .borrow()
            .keys()
            .filter(|path| path.starts_with(root))
            .cloned()
            .collect())
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        self.files
            .borrow()
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.files
            .borrow_mut()
            .insert(path.to_path_buf(), contents.to_string());
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = self.files.borrow_mut();
        let contents = files
            .remove(from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "not found"))?;
        files.insert(to.to_path_buf(), contents);
        Ok(())
    }
}
