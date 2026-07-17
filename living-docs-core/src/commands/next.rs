use crate::store::DocStore;
use std::fs;
use std::io;
use std::path::Path;
use std::process::ExitCode;

pub fn run(docs_dir: &Path, doc_type: &str) -> ExitCode {
    match next_number(docs_dir, doc_type) {
        Ok(n) => {
            println!("{n:04}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("living-docs next: {e}");
            ExitCode::from(2)
        }
    }
}

/// Highest existing `NNNN` under `docs_dir/doc_type`, plus one. `doc_type`
/// here is the resolved directory name (e.g. `issues`, not `issue`) — `new`
/// reuses this to avoid duplicating the allocation logic.
pub fn next_number(docs_dir: &Path, doc_type: &str) -> std::io::Result<u32> {
    let type_dir = docs_dir.join(doc_type);
    let highest = match fs::read_dir(&type_dir) {
        Ok(entries) => highest_numeric_prefix(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
        Err(e) => return Err(e),
    };
    Ok(highest + 1)
}

fn highest_numeric_prefix(entries: fs::ReadDir) -> u32 {
    entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| numeric_prefix(&entry.file_name().to_string_lossy()))
        .max()
        .unwrap_or(0)
}

/// Backend-agnostic sibling of [`next_number`]: the highest existing `NNNN`
/// under `root`'s `doc_type` directory, plus one, computed from
/// [`DocStore::list`] instead of `std::fs` — so `new` allocates correctly
/// whichever backend is authoritative (issue 0006 slice 0006-C2). `root` is
/// the bundle root `store.list` was rooted at; `doc_type` is the resolved
/// directory name, matching [`next_number`]. Reuses [`numeric_prefix`], the
/// same four-digit-prefix parsing `next_number` itself uses.
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

fn numeric_prefix(filename: &str) -> Option<u32> {
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
    use std::time::{SystemTime, UNIX_EPOCH};

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

    struct TempFsStore;

    impl DocStore for TempFsStore {
        fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
            let mut found = Vec::new();
            collect_markdown_files(root, &mut found);
            found.sort();
            Ok(found)
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            fs::read_to_string(path)
        }

        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, contents)
        }
    }

    fn collect_markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                collect_markdown_files(&path, out);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                out.push(path);
            }
        }
    }

    fn temp_bundle(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("living-docs-core-next-{label}-{nanos}"))
    }

    #[test]
    fn next_number_from_store_matches_next_number_on_the_same_fs_backed_directory() {
        let bundle = temp_bundle("fs-parity");
        let type_dir = bundle.join("adr");
        fs::create_dir_all(&type_dir).expect("create type dir");
        fs::write(type_dir.join("0001-first.md"), "content").expect("write fixture");
        fs::write(type_dir.join("0004-fourth.md"), "content").expect("write fixture");

        let via_fs = next_number(&bundle, "adr").expect("next_number should succeed");
        let via_store = next_number_from_store(&TempFsStore, &bundle, "adr")
            .expect("next_number_from_store should succeed");

        assert_eq!(via_fs, via_store);
        assert_eq!(via_store, 5);

        let _ = fs::remove_dir_all(&bundle);
    }
}
