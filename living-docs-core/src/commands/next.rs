use crate::store::DocStore;
use std::io;
use std::path::Path;

/// The highest existing `NNNN` under `root`'s `doc_type` directory, plus one,
/// computed from [`DocStore::list`] so `new` allocates correctly whichever
/// backend is authoritative. `root` is the bundle root `store.list` was
/// rooted at; `doc_type` is the resolved directory name (e.g. `issues`, not
/// `issue`).
pub fn next_number_from_store(
    store: &dyn DocStore,
    root: &Path,
    doc_type: &str,
) -> io::Result<u32> {
    let paths = store.list(root)?;
    let highest = paths
        .iter()
        .filter(|path| is_in_type_dir(path, doc_type))
        .filter_map(|path| path.file_name())
        .filter_map(|name| numeric_prefix(&name.to_string_lossy()))
        .max()
        .unwrap_or(0);
    Ok(highest + 1)
}

fn is_in_type_dir(path: &Path, doc_type: &str) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == doc_type)
}

/// The `NNNN` of a `NNNN-*.md` filename, `None` for any other name.
pub(crate) fn numeric_prefix(filename: &str) -> Option<u32> {
    if !filename.ends_with(".md") || filename.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let prefix = filename.get(0..4)?;
    if !prefix.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    prefix.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::io;
    use std::path::PathBuf;

    #[test]
    fn numeric_prefix_accepts_four_digit_dash_form() {
        assert_eq!(numeric_prefix("0007-old.md"), Some(7));
    }

    #[test]
    fn numeric_prefix_rejects_non_matching_filenames() {
        assert_eq!(numeric_prefix("index.md"), None);
        assert_eq!(numeric_prefix("notes.txt"), None);
        assert_eq!(numeric_prefix("12-old.md"), None);
        assert_eq!(numeric_prefix("abcd-old.md"), None);
    }

    struct FixtureStore {
        paths: BTreeSet<PathBuf>,
    }

    impl DocStore for FixtureStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            Ok(self
                .paths
                .iter()
                .filter(|path| path.starts_with(root))
                .cloned()
                .collect())
        }

        fn read(&self, _path: &Path) -> io::Result<String> {
            Err(io::Error::new(io::ErrorKind::NotFound, "unused in fixture"))
        }

        fn write(&self, _path: &Path, _contents: &str) -> io::Result<()> {
            Ok(())
        }

        fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn next_number_from_store_returns_one_when_the_type_dir_has_no_entries() {
        let store = FixtureStore {
            paths: BTreeSet::new(),
        };
        let next = next_number_from_store(&store, Path::new("/bundle"), "adr")
            .expect("next_number_from_store should succeed on an empty store");
        assert_eq!(next, 1);
    }

    #[test]
    fn next_number_from_store_returns_the_highest_existing_prefix_plus_one() {
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("/bundle/adr/0001-first.md"));
        paths.insert(PathBuf::from("/bundle/adr/0003-third.md"));
        paths.insert(PathBuf::from("/bundle/bdr/0009-unrelated.md"));
        let store = FixtureStore { paths };

        let next = next_number_from_store(&store, Path::new("/bundle"), "adr")
            .expect("next_number_from_store should succeed");

        assert_eq!(next, 4);
    }

    #[test]
    fn next_number_from_store_ignores_entries_outside_the_doc_type_directory() {
        let mut paths = BTreeSet::new();
        paths.insert(PathBuf::from("/bundle/adr/0001-first.md"));
        paths.insert(PathBuf::from("/bundle/issues/0099-unrelated.md"));
        let store = FixtureStore { paths };

        let next = next_number_from_store(&store, Path::new("/bundle"), "adr")
            .expect("next_number_from_store should succeed");

        assert_eq!(next, 2);
    }
}
