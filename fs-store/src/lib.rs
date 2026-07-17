//! `DocStore` adapter over `.md` files on disk (ADR 0002, slice S0b1). Mirrors
//! `living_docs_core::check`'s doc-enumeration semantics: recursive walk,
//! `.md` files only, sorted, lenient on unreadable subdirectories.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use living_docs_core::store::DocStore;

/// `DocStore` implementation backed by the local filesystem.
#[derive(Debug, Default, Clone, Copy)]
pub struct FsStore;

impl FsStore {
    /// Builds an `FsStore`. Holds no state — every call is rooted at the
    /// `root`/`path` argument it receives.
    pub fn new() -> Self {
        Self
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
        fs::read_to_string(path)
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TempTestDir {
        path: PathBuf,
    }

    impl TempTestDir {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!("fs-store-test-{label}-{nanos}"));
            fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }
    }

    impl Drop for TempTestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn list_enumerates_only_md_files_in_sorted_order() {
        let temp = TempTestDir::new("list-sorted");
        let root = temp.path.clone();
        fs::create_dir_all(root.join("nested")).expect("create nested dir");
        fs::write(root.join("b.md"), "b").expect("write b.md");
        fs::write(root.join("a.md"), "a").expect("write a.md");
        fs::write(root.join("notes.txt"), "not markdown").expect("write notes.txt");
        fs::write(root.join("nested").join("c.md"), "c").expect("write nested c.md");

        let store = FsStore::new();
        let found = store.list(&root).expect("list should succeed");

        assert_eq!(
            found,
            vec![
                root.join("a.md"),
                root.join("b.md"),
                root.join("nested").join("c.md"),
            ]
        );
    }

    #[test]
    fn list_on_missing_root_returns_empty_result() {
        let temp = TempTestDir::new("list-missing-root");
        let missing_root = temp.path.join("does-not-exist");

        let store = FsStore::new();
        let found = store
            .list(&missing_root)
            .expect("missing root should be a lenient empty list, not an error");

        assert!(found.is_empty());
    }

    #[test]
    fn write_then_read_round_trips_contents() {
        let temp = TempTestDir::new("round-trip");
        let target = temp.path.join("doc.md");
        let store = FsStore::new();

        store
            .write(&target, "# Title\n\nBody text.\n")
            .expect("write should succeed");
        let read_back = store.read(&target).expect("read should succeed");

        assert_eq!(read_back, "# Title\n\nBody text.\n");
    }

    #[test]
    fn write_creates_missing_parent_directories() {
        let temp = TempTestDir::new("write-parents");
        let target = temp.path.join("adr").join("nested").join("0001-title.md");
        let store = FsStore::new();

        store
            .write(&target, "content")
            .expect("write should create missing parents and succeed");

        assert!(target.is_file());
        assert_eq!(
            fs::read_to_string(&target).expect("read written file"),
            "content"
        );
    }
}
