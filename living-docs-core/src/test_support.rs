//! Shared `#[cfg(test)]` fixtures for `check`'s unit tests: a single
//! `MapStore` (a [`crate::store::DocStore`] backed by an in-memory
//! `BTreeMap`, with no filesystem I/O) reused by `check::mod`,
//! `check::records`, and `check::canonical` instead of three near-identical
//! copies of the same fixture.

use crate::store::DocStore;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub(crate) struct MapStore {
    pub(crate) files: BTreeMap<PathBuf, String>,
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
}
