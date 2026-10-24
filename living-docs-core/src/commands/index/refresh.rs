use super::regenerate;
use crate::paths;
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// One index that could not be written after the record it lists was
/// already written. `doc_type` is the token `living-docs index` repairs, or
/// `None` when the failing file is the bundle-root `index.md`, which no
/// per-type `index` run can repair.
#[derive(Debug)]
pub struct IndexFailure {
    pub doc_type: Option<&'static str>,
    pub error: String,
}

/// The repair line a front prints after its verb prefix: the record is
/// written, the index is not, and the named verb fixes it. A bundle-root
/// failure asks for the root path or its directory to be fixed before
/// `living-docs index` writes it again, since no per-type run repairs it.
impl std::fmt::Display for IndexFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.doc_type {
            Some(doc_type) => write!(
                f,
                "record written; index not updated ({}) — run `living-docs index {doc_type}`",
                self.error
            ),
            None => write!(
                f,
                "record written; the bundle-root index could not be written ({}) — fix that path or its directory, then run `living-docs index`",
                self.error
            ),
        }
    }
}

/// What a mutation verb's index refresh did: the index files it rewrote and
/// the types it could not. A failure never undoes the record write, so the
/// front reports it apart from a usage error.
#[derive(Debug, Default)]
pub struct Indexed {
    pub written: Vec<PathBuf>,
    pub failures: Vec<IndexFailure>,
}

/// The type token whose directory directly holds `record`, `None` for a file
/// at the bundle root (a singleton such as the constitution has no index).
fn type_of_record(docs_dir: &Path, record: &Path) -> Option<&'static str> {
    let dir = record.parent()?;
    if dir.parent()? != docs_dir {
        return None;
    }
    paths::doc_type_for_dir(dir.file_name()?.to_str()?)
}

/// Regenerates the index of the type `record` belongs to, writing only when
/// the content differs from the file on disk; `None` when nothing was written
/// or the record has no type directory.
pub fn regenerate_for_record(
    store: &dyn DocStore,
    docs_dir: &Path,
    record: &Path,
) -> Result<Option<PathBuf>, String> {
    match type_of_record(docs_dir, record) {
        Some(doc_type) => regenerate(store, docs_dir, doc_type),
        None => Ok(None),
    }
}

/// Refreshes the index of every type the touched `records` belong to, once
/// per type, collecting what was written and what failed instead of stopping
/// at the first error.
pub fn refresh_for_records(store: &dyn DocStore, docs_dir: &Path, records: &[&Path]) -> Indexed {
    let mut types: Vec<&'static str> = records
        .iter()
        .filter_map(|record| type_of_record(docs_dir, record))
        .collect();
    types.sort_unstable();
    types.dedup();
    let mut indexed = Indexed::default();
    for doc_type in types {
        match regenerate(store, docs_dir, doc_type) {
            Ok(Some(path)) => indexed.written.push(path),
            Ok(None) => {}
            Err(error) => indexed.failures.push(IndexFailure {
                doc_type: Some(doc_type),
                error,
            }),
        }
    }
    indexed
}
