use super::*;
use crate::store::RepositoryFiles;
use crate::test_support::MapStore;

/// A store answering both repository questions, to prove a wrapper forwards them.
struct Probing(MapStore);

impl DocStore for Probing {
    fn list(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        self.0.list(root)
    }
    fn read(&self, path: &Path) -> io::Result<String> {
        self.0.read(path)
    }
    fn write(&self, path: &Path, text: &str) -> io::Result<()> {
        self.0.write(path, text)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.0.rename(from, to)
    }
    fn repository_entry_exists(&self, _: &Path) -> Option<bool> {
        Some(true)
    }
    fn repository_files(&self, globs: &[String]) -> Option<RepositoryFiles> {
        self.0.repository_files(globs)
    }
}

#[test]
fn a_draft_store_passes_the_entry_probe_through() {
    let inner = Probing(MapStore::seeded(&[]).0);
    let drafted = Drafted {
        inner: &inner,
        drafted: &[],
    };
    assert_eq!(drafted.repository_entry_exists(Path::new("a")), Some(true));
}

#[test]
fn a_draft_store_passes_the_repository_files_through() {
    let inner = Probing(MapStore::seeded(&[("a.txt", "x")]).0);
    let drafted = Drafted {
        inner: &inner,
        drafted: &[],
    };
    let files = drafted.repository_files(&["*.txt".to_owned()]);
    assert_eq!(files.expect("served").unwrap().len(), 1);
}
