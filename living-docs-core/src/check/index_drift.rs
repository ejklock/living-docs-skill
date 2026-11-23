//! Index drift: a type's `index.md` that differs from what `index` would
//! regenerate for it. A merge or a hand edit can leave a duplicated row, a
//! stale status or rows out of order; the CLI never does, because `index` is
//! idempotent. The comparison reuses the computation `index` writes with, in
//! memory and without writing, so a preamble `index` keeps counts as written.

use super::Reporter;
use crate::commands::index::pending;
use crate::doc_type::{self, Identity};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// The index followed by every record under its type's directory, so a scope
/// holding the index or any record of the type keeps the finding.
fn anchors(store: &dyn DocStore, type_dir: PathBuf, index: PathBuf) -> Vec<PathBuf> {
    let records = store.list(&type_dir).unwrap_or_default();
    std::iter::once(index).chain(records).collect()
}

/// Reports a violation for every existing type index that differs from its
/// regeneration, naming the index and the `living-docs manage index <type>` repair.
/// A type with no directory, or whose directory has no `index.md`, is left to
/// the membership checks. The violation is anchored on the index and on every
/// record of the type, so `--changed-files` naming one of them keeps it.
pub(crate) fn check_index_drift(store: &dyn DocStore, bundle: &Path, reporter: &mut Reporter) {
    for spec in doc_type::DOC_TYPES {
        let (Identity::Numbered { dir } | Identity::Named { dir }) = spec.identity else {
            continue;
        };
        let index = bundle.join(dir).join("index.md");
        if !index.is_file() {
            continue;
        }
        if matches!(pending(store, bundle, spec.token), Ok(Some(_))) {
            reporter.report_group(
                &anchors(store, bundle.join(dir), index),
                format!(
                    "index differs from its regeneration — run `living-docs manage index {}`",
                    spec.token
                ),
            );
        }
    }
}
