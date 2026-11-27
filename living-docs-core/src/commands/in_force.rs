//! The records in force of a bundle: the one rule that decides which files
//! are records and which of them their doc type has retired. Every front that
//! lists or reads records goes through it, so none can disagree.

use crate::doc_type;
use crate::record::{self, ExtractedRecord};
use crate::store::DocStore;
use std::path::{Path, PathBuf};

/// A record paired with the path of its file.
pub type Located = (PathBuf, ExtractedRecord);

/// Every record under `bundle` that its doc type has not retired, with the
/// path of its file; `index.md` and `log.md` are never records. Empty when
/// the bundle cannot be listed.
pub fn in_force_records(store: &dyn DocStore, bundle: &Path) -> Vec<Located> {
    let mut records = typed_records(store, &store.list(bundle).unwrap_or_default());
    records.retain(|(_, record)| is_in_force(record));
    records
}

/// Every record among `all_md`, retired ones included.
pub(crate) fn typed_records(store: &dyn DocStore, all_md: &[PathBuf]) -> Vec<Located> {
    all_md
        .iter()
        .filter(|path| !is_reserved(path))
        .filter_map(|path| {
            let contents = store.read(path).ok()?;
            let record = record::extract_record(path, &contents);
            (!record.doc_type.is_empty()).then(|| (path.clone(), record))
        })
        .collect()
}

fn is_reserved(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("index.md") | Some("log.md")
    )
}

/// Whether the record's doc type retires its status; an unregistered type or
/// a missing status is live.
pub(crate) fn is_in_force(record: &ExtractedRecord) -> bool {
    let retired = record.status.as_deref().is_some_and(|status| {
        doc_type::spec_for_frontmatter(&record.doc_type).is_some_and(|spec| spec.is_retired(status))
    });
    !retired
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::MapStore;

    fn record(kind: &str, status: &str) -> String {
        format!("---\ntype: {kind}\ntitle: T\ndescription: d\nstatus: {status}\n---\n\nBody.\n")
    }

    #[test]
    fn in_force_records_keep_live_records_and_drop_retired_and_reserved_files() {
        let (store, _) = MapStore::seeded(&[
            ("bundle/adr/0001-live.md", &record("ADR", "Accepted")),
            ("bundle/adr/0002-old.md", &record("ADR", "Superseded")),
            ("bundle/work/0001-done.md", &record("Issue", "closed")),
            ("bundle/work/0002-open.md", &record("Issue", "open")),
            ("bundle/adr/index.md", &record("ADR", "Accepted")),
            ("bundle/log.md", &record("ADR", "Accepted")),
            ("bundle/notes/plain.md", "no frontmatter\n"),
        ]);

        let paths: Vec<PathBuf> = in_force_records(&store, Path::new("bundle"))
            .into_iter()
            .map(|(path, _)| path)
            .collect();

        assert_eq!(
            paths,
            [
                PathBuf::from("bundle/adr/0001-live.md"),
                PathBuf::from("bundle/work/0002-open.md")
            ]
        );
    }
}
