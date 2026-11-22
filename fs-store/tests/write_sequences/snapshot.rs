//! A bundle's bytes, read and copied with std only.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// What a bundle path holds: a directory, or a file with its bytes.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Entry {
    Dir,
    File(Vec<u8>),
}

/// Every directory and file of a bundle by its bundle-relative path.
pub type Snapshot = BTreeMap<PathBuf, Entry>;

/// Reads every directory and file under `root` into a [`Snapshot`].
pub fn take(root: &Path) -> Snapshot {
    let mut files = Snapshot::new();
    collect(root, root, &mut files);
    files
}

fn collect(root: &Path, dir: &Path, files: &mut Snapshot) {
    for entry in fs::read_dir(dir).expect("list bundle directory") {
        let path = entry.expect("read bundle entry").path();
        let relative = path.strip_prefix(root).expect("path under root");
        if path.is_dir() {
            files.insert(relative.to_path_buf(), Entry::Dir);
            collect(root, &path, files);
        } else {
            let bytes = fs::read(&path).expect("read bundle file");
            files.insert(relative.to_path_buf(), Entry::File(bytes));
        }
    }
}

/// Copies the tree at `from` to `to`, creating `to`.
pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create copy directory");
    for entry in fs::read_dir(from).expect("list directory to copy") {
        let path = entry.expect("read entry to copy").path();
        let target = to.join(path.file_name().expect("entry name"));
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            fs::copy(&path, &target).expect("copy bundle file");
        }
    }
}

/// The paths whose content differs between two snapshots, or that exist in only one.
pub fn changed(before: &Snapshot, after: &Snapshot) -> Vec<PathBuf> {
    let paths: BTreeSet<&PathBuf> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}
